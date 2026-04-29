#!/usr/bin/env bash
# scripts/deploy.sh
#
# Interactive deploy script for dataprovider_staking.
#
# Flow:
#   1. Preflight checks (keypairs present, SOL balance, USDC mint decimals, etc.)
#   2. Confirm target cluster (devnet / mainnet-beta).
#   3. anchor build (produces IDL + .so with the correct program id for the cluster).
#   4. solana program deploy, using the admin wallet as both payer and
#      upgrade authority.
#   5. Call `initialize` with admin wallet as both payer and admin.
#   6. Optionally call `add-pool` for $GHC1CHEM.
#
# Usage:
#   scripts/deploy.sh devnet
#   scripts/deploy.sh mainnet-beta
#
# Flags:
#   --yes       Skip interactive confirmations (DANGEROUS on mainnet).
#   --skip-pool Skip the add-pool step.

set -euo pipefail

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

CLUSTER="${1:-}"
shift || true

YES_FLAG=0
SKIP_POOL=0
for arg in "$@"; do
    case "$arg" in
        --yes) YES_FLAG=1 ;;
        --skip-pool) SKIP_POOL=1 ;;
    esac
done

if [[ -z "$CLUSTER" ]]; then
    echo "Usage: $0 <devnet|mainnet-beta> [--yes] [--skip-pool]"
    exit 1
fi

case "$CLUSTER" in
    devnet)
        RPC_URL="https://api.devnet.solana.com"
        PROGRAM_KEYPAIR="$ROOT_DIR/target/deploy/dataprovider_staking-devnet-keypair.json"
        ANCHOR_CLUSTER="devnet"
        ;;
    mainnet|mainnet-beta)
        RPC_URL="https://api.mainnet-beta.solana.com"
        PROGRAM_KEYPAIR="$ROOT_DIR/keys/program-mainnet-keypair.json"
        ANCHOR_CLUSTER="mainnet"
        ;;
    *)
        echo "ERROR: unknown cluster '$CLUSTER' (expected devnet or mainnet-beta)"
        exit 1
        ;;
esac

ADMIN_KEYPAIR="$ROOT_DIR/keys/admin-wallet.json"
EXPECTED_ADMIN="6HGeNL5852ykqQNiwT6sC5YFu1xBBwvgtVnUWuf5EfEP"
USDC_MAINNET="EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
GHC1CHEM_MINT="3pi9trvC6hrMUHHhQnQy5aAPk5CzxAGxsLyiXzshpump"

log() { printf "\033[1;34m[deploy]\033[0m %s\n" "$*"; }
warn() { printf "\033[1;33m[deploy]\033[0m %s\n" "$*"; }
err() { printf "\033[1;31m[deploy]\033[0m %s\n" "$*" >&2; exit 1; }

confirm() {
    if [[ "$YES_FLAG" == "1" ]]; then return 0; fi
    read -r -p "$1 [y/N]: " reply
    [[ "$reply" =~ ^[Yy]$ ]] || { echo "aborted."; exit 1; }
}

# ─── Preflight ────────────────────────────────────────────────────────────────
log "target cluster: $CLUSTER ($RPC_URL)"

[[ -f "$ADMIN_KEYPAIR" ]] || err "missing $ADMIN_KEYPAIR (place the 6HGeNL... keypair there)"
[[ -f "$PROGRAM_KEYPAIR" ]] || err "missing $PROGRAM_KEYPAIR"

ACTUAL_ADMIN="$(solana-keygen pubkey "$ADMIN_KEYPAIR")"
if [[ "$ACTUAL_ADMIN" != "$EXPECTED_ADMIN" ]]; then
    err "admin keypair pubkey mismatch: got $ACTUAL_ADMIN, expected $EXPECTED_ADMIN"
fi
log "admin pubkey ok: $ACTUAL_ADMIN"

PROGRAM_ID="$(solana-keygen pubkey "$PROGRAM_KEYPAIR")"
log "program id:     $PROGRAM_ID"

# Pull the compiled declare_id! out of the IDL or grep the source as a sanity check.
DECLARED_ID="$(grep -E '^declare_id!' programs/dataprovider_staking/src/lib.rs | sed -E 's/.*"([^"]+)".*/\1/')"
if [[ "$CLUSTER" == "mainnet" || "$CLUSTER" == "mainnet-beta" ]]; then
    [[ "$DECLARED_ID" == "$PROGRAM_ID" ]] || err "declare_id! ($DECLARED_ID) != mainnet program id ($PROGRAM_ID). Update lib.rs and rebuild."
fi
log "declare_id! ok: $DECLARED_ID"

# SOL balance check
BALANCE_LAMPORTS="$(solana balance --url "$RPC_URL" --output json "$ACTUAL_ADMIN" 2>/dev/null | tr -d '\n' || echo '0 SOL')"
BALANCE_SOL="$(solana balance --url "$RPC_URL" "$ACTUAL_ADMIN" 2>/dev/null | awk '{print $1}' || echo '0')"
log "admin SOL balance on $CLUSTER: $BALANCE_SOL"

# Program rent for a 351480-byte .so ≈ 2.45 SOL. Require 3.
MIN_SOL=3
if awk -v b="$BALANCE_SOL" -v m="$MIN_SOL" 'BEGIN { exit (b+0 < m+0) ? 0 : 1 }'; then
    warn "admin wallet has < ${MIN_SOL} SOL on $CLUSTER — deploy will likely fail (program rent ~2.5 SOL)."
    confirm "Continue anyway?"
fi

# Confirm USDC mint on mainnet (can't reach chain on devnet for this in a generic way).
if [[ "$CLUSTER" == "mainnet" || "$CLUSTER" == "mainnet-beta" ]]; then
    log "USDC mint (mainnet): $USDC_MAINNET  (will be baked into GlobalConfig.usdc_mint permanently)"

    # Verify USDC & GHC1CHEM token program ownership. The program supports
    # both SPL Token and Token-2022 via the token_interface refactor, but we
    # want to confirm the caller knows which program will be used.
    USDC_OWNER="$(solana account "$USDC_MAINNET" --url "$RPC_URL" --output json 2>/dev/null | python3 -c 'import json,sys;print(json.load(sys.stdin)["account"]["owner"])')"
    GHC_OWNER="$(solana account "$GHC1CHEM_MINT" --url "$RPC_URL" --output json 2>/dev/null | python3 -c 'import json,sys;print(json.load(sys.stdin)["account"]["owner"])')"
    SPL_CLASSIC="TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
    TOKEN_2022="TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"

    describe_program() {
        case "$1" in
            "$SPL_CLASSIC") echo "SPL Token (classic)" ;;
            "$TOKEN_2022") echo "Token-2022" ;;
            *) echo "UNKNOWN ($1)" ;;
        esac
    }

    log "USDC mint owner:      $USDC_OWNER  [$(describe_program "$USDC_OWNER")]"
    log "GHC1CHEM mint owner:  $GHC_OWNER  [$(describe_program "$GHC_OWNER")]"

    if [[ "$USDC_OWNER" != "$SPL_CLASSIC" && "$USDC_OWNER" != "$TOKEN_2022" ]]; then
        err "USDC mint is owned by an unknown program ($USDC_OWNER). Aborting."
    fi
    if [[ "$GHC_OWNER" != "$SPL_CLASSIC" && "$GHC_OWNER" != "$TOKEN_2022" ]]; then
        err "GHC1CHEM mint is owned by an unknown program ($GHC_OWNER). Aborting."
    fi
fi

echo
log "Summary:"
echo "  cluster:            $CLUSTER"
echo "  program id:         $PROGRAM_ID"
echo "  program keypair:    $PROGRAM_KEYPAIR"
echo "  admin / payer:      $ACTUAL_ADMIN"
echo "  admin keypair:      $ADMIN_KEYPAIR"
echo "  upgrade authority:  $ACTUAL_ADMIN (default; same as payer)"
echo
confirm "Proceed with deploy?"

# ─── Build ────────────────────────────────────────────────────────────────────
log "anchor build --provider.cluster $ANCHOR_CLUSTER"
anchor build --provider.cluster "$ANCHOR_CLUSTER"

# Copy the right program keypair into target/deploy so anchor deploy uses it.
cp "$PROGRAM_KEYPAIR" "$ROOT_DIR/target/deploy/dataprovider_staking-keypair.json"

# ─── Deploy ───────────────────────────────────────────────────────────────────
log "solana program deploy (this can take ~1 min)…"
solana program deploy \
    "$ROOT_DIR/target/deploy/dataprovider_staking.so" \
    --program-id "$PROGRAM_KEYPAIR" \
    --keypair "$ADMIN_KEYPAIR" \
    --upgrade-authority "$ADMIN_KEYPAIR" \
    --url "$RPC_URL"

log "program deployed. verifying on-chain…"
solana program show "$PROGRAM_ID" --url "$RPC_URL"

# ─── Initialize ───────────────────────────────────────────────────────────────
log "initialize GlobalConfig (admin = $ACTUAL_ADMIN, usdc = $USDC_MAINNET)"

# The CLI expects a keypair path; point it at the admin keypair so
# `initialize` is signed by the admin directly, making admin == signer.
if [[ "$CLUSTER" == "mainnet" || "$CLUSTER" == "mainnet-beta" ]]; then
    USDC_FOR_INIT="$USDC_MAINNET"
else
    # devnet: prompt for a USDC mint (a mock or the devnet USDC you control).
    read -r -p "Enter USDC mint to use for devnet initialize: " USDC_FOR_INIT
fi

pushd "$ROOT_DIR/scripts" >/dev/null
npx ts-node src/cli.ts \
    --cluster "$CLUSTER" \
    --keypair "$ADMIN_KEYPAIR" \
    --program-id "$PROGRAM_ID" \
    initialize \
    --usdc-mint "$USDC_FOR_INIT"
popd >/dev/null

# ─── Add pool ─────────────────────────────────────────────────────────────────
if [[ "$SKIP_POOL" == "1" ]]; then
    log "skipping add-pool per --skip-pool"
else
    confirm "Add $GHC1CHEM_MINT pool now?"
    pushd "$ROOT_DIR/scripts" >/dev/null
    npx ts-node src/cli.ts \
        --cluster "$CLUSTER" \
        --keypair "$ADMIN_KEYPAIR" \
        --program-id "$PROGRAM_ID" \
        add-pool \
        --stake-mint "$GHC1CHEM_MINT"
    popd >/dev/null
fi

# ─── Summary ──────────────────────────────────────────────────────────────────
log "deploy complete"
echo
echo "  program id:        $PROGRAM_ID"
echo "  explorer:          https://explorer.solana.com/address/$PROGRAM_ID?cluster=$CLUSTER"
echo "  upgrade authority: $ACTUAL_ADMIN"
echo "  in-program admin:  $ACTUAL_ADMIN"
echo
log "next steps:"
echo "  • Verify GlobalConfig: npm --prefix scripts run cli -- --cluster $CLUSTER --keypair $ADMIN_KEYPAIR --program-id $PROGRAM_ID status"
echo "  • Deposit rewards:    scripts/src/cli.ts deposit-rewards --stake-mint $GHC1CHEM_MINT --amount <lamports>"
echo "  • If you want to rotate upgrade authority to a multisig, run:"
echo "      solana program set-upgrade-authority $PROGRAM_ID --keypair $ADMIN_KEYPAIR --new-upgrade-authority <MULTISIG_PUBKEY> --url $RPC_URL"
