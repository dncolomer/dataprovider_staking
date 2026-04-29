# dataprovider_staking

Multi-mint Solana staking program with USDC dividend distribution.

- **Stake**: users deposit tokens from any registered SPL mint. Pool 1 is
  `$GHC1CHEM` (`3pi9trvC6hrMUHHhQnQy5aAPk5CzxAGxsLyiXzshpump`). Up to **5**
  pools can be added.
- **Rewards**: the admin manually deposits USDC into a pool. Each USDC
  deposit is distributed pro-rata to that pool's stakers at the moment of
  deposit (classic MasterChef `acc_reward_per_share` accumulator scaled by
  `1e12`). Pools have independent USDC vaults.
- **Claims**: any user can claim their pending USDC at any time. Unstaking
  and staking automatically settle pending rewards first.
- **Admin**: a single authority in `GlobalConfig`. Target production admin:
  `6HGeNL5852ykqQNiwT6sC5YFu1xBBwvgtVnUWuf5EfEP`. Rotation is 2-step
  (`propose_admin` → `accept_admin`) to prevent transferring to a dead key.

## Repo layout

```
programs/dataprovider_staking/   Anchor program (Rust)
sdk/                             TypeScript SDK (shared by CLI + web)
scripts/                         Admin CLI (Node)
web/                             Next.js investor frontend
target/                          build artifacts (idl, so, keypair)
```

## Prereqs

- Rust `1.89+` (toolchain pinned via `rust-toolchain.toml`)
- `anchor-cli 1.0.0`
- `solana-cli 3.x` (tested with 3.1.13)
- Node `20+`, `npm`

## Local test loop (no deploy needed)

All on-chain tests run inside [LiteSVM](https://github.com/LiteSVM/litesvm)
with the freshly-compiled program binary — no validator, no RPC, no funding.

```
anchor build
cargo test -p dataprovider_staking --release
```

You should see **26/26 tests green**: `test_initialize` (5) and
`test_staking_flow` (20), covering:

- initialize sets admin / usdc mint
- 2-step admin rotation (propose / accept / cancel / interloper rejected / no-pending rejected)
- only admin can `add_pool`; 6th pool rejected (`MaxPoolsReached`)
- stake, partial unstake, full unstake return principal
- unstake zero rejected (`ZeroAmount`); unstake past balance rejected (`InsufficientStake`)
- deposits fail when pool has zero stakers
- zero-amount deposit rejected (`ZeroAmount`)
- dust deposit (would round to zero) rejected (`RewardDepositTooSmall`)
- wrong stake vault / wrong reward vault rejected (`InvalidStakeMint` / `InvalidRewardMint`)
- claim fails when pending == 0
- single-staker earns 100% of rewards
- two stakers split rewards 1:3
- late stakers only earn on *future* deposits
- unstaking preserves pending rewards for later claim
- one user staking in multiple pools earns independently per pool
- adding stake after a deposit does NOT backfill past rewards
- sequential deposits accumulate correctly
- `UserStake` PDA reuse after full unstake

## Local validator end-to-end (optional)

Spin up a validator, deploy, and drive through the admin CLI:

```sh
# 1. validator
solana-test-validator --reset --quiet &

# 2. build + deploy
anchor build
solana program deploy target/deploy/dataprovider_staking.so \
  --program-id target/deploy/dataprovider_staking-keypair.json

# 3. mock mints (local stand-ins for USDC + GHC1CHEM)
USDC=$(spl-token create-token --decimals 6 | awk '/Address:/{print $2}')
STAKE=$(spl-token create-token --decimals 9 | awk '/Address:/{print $2}')
spl-token create-account $USDC
spl-token mint $USDC 1000

# 4. initialize + add pool via the admin CLI
npm install
npm run build
node scripts/dist/cli.js initialize --usdc-mint $USDC
node scripts/dist/cli.js add-pool --stake-mint $STAKE
node scripts/dist/cli.js status

# 5. frontend
npm run dev:web   # http://localhost:3000
```

## Admin CLI reference

All commands accept `--cluster`, `--keypair`, `--program-id`.

```
dps-admin initialize        --usdc-mint <pk>
dps-admin add-pool          --stake-mint <pk>
dps-admin deposit-rewards   --stake-mint <pk> --amount <u64>
dps-admin propose-admin     --new-admin <pk>
dps-admin accept-admin               # run as the new admin
dps-admin cancel-admin
dps-admin status
dps-admin pool              --stake-mint <pk>
```

Example: deposit 100 USDC (6 decimals → `100_000_000`) into the $GHC1CHEM pool:

```sh
node scripts/dist/cli.js deposit-rewards \
  --stake-mint 3pi9trvC6hrMUHHhQnQy5aAPk5CzxAGxsLyiXzshpump \
  --amount   100000000
```

## On-chain data model

- **`GlobalConfig`** (singleton, seeds `["config"]`)
  `admin`, `pending_admin`, `usdc_mint`, `pool_count`.

- **`TokenPool`** (per stake-mint, seeds `["pool", stake_mint]`)
  `stake_mint`, `stake_vault`, `reward_vault`, `total_staked`,
  `acc_reward_per_share` (u128, ×1e12), `total_rewards_deposited`,
  `total_rewards_claimed`, `vault_authority_bump`.

- **`UserStake`** (per user+pool, seeds `["user", stake_mint, owner]`)
  `owner`, `stake_mint`, `amount`, `reward_debt`, `pending_rewards`,
  `total_claimed`.

- **Vaults**: two SPL token accounts per pool (`["stake_vault", mint]`,
  `["reward_vault", mint]`), both owned by a PDA `["vault_auth", mint]`.

## Reward math

When admin deposits `Δ` USDC into a pool with `S` currently staked:

```
acc_reward_per_share += Δ * 1e12 / S
```

Each user's unsettled earnings are:

```
earned = user.amount * pool.acc / 1e12 - user.reward_debt
```

On any state-changing call we first fold `earned` into `pending_rewards`,
then update `reward_debt` to the new basis. Precision is ample: with
`u128` accumulator and `1e12` scaling, a pool can absorb ~3.4e26 USDC
before overflow risk (practically unbounded).

## Deploying to devnet / mainnet

A single script (`scripts/deploy.sh`) handles everything: preflight checks
(keypair presence & pubkey match, SOL balance, declared program id),
build, program deploy (payer + upgrade authority = admin wallet),
`initialize`, and optional `add-pool` for $GHC1CHEM.

### Prereqs

1. Place the admin (`6HGeNL5852ykqQNiwT6sC5YFu1xBBwvgtVnUWuf5EfEP`)
   keypair at `keys/admin-wallet.json`. The `keys/` directory is
   gitignored.
2. Ensure the admin wallet is funded with at least ~3 SOL on the target
   cluster (program rent ≈ 2.5 SOL + tx fees).
3. Install deps: `npm install && npm --prefix scripts install`.
4. Verify tests pass locally: `cargo test -p dataprovider_staking --release`.

### Devnet dry-run (recommended)

```sh
scripts/deploy.sh devnet
```

You'll be prompted for a USDC mint to bake into `GlobalConfig`
(use a devnet USDC mint or a mock mint you control).

### Mainnet deploy

```sh
scripts/deploy.sh mainnet-beta
```

Uses `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` (official USDC) and
the `3pi9trvC6hrMUHHhQnQy5aAPk5CzxAGxsLyiXzshpump` $GHC1CHEM mint.

### Program IDs

| Env       | Program ID                                       | Keypair file                                      |
| --------- | ------------------------------------------------ | ------------------------------------------------- |
| Mainnet   | `94Ja6Y8AuzmZHjQiyk2SzvoysnBr3F17nfHGrHm1idAZ`   | `keys/program-mainnet-keypair.json`               |
| Devnet    | `AnConH6PVX1UQYtdPgAgUNMowphcragEjbGsx3nQJ6up`   | `target/deploy/dataprovider_staking-devnet-keypair.json` |
| Localnet  | `94Ja6Y8AuzmZHjQiyk2SzvoysnBr3F17nfHGrHm1idAZ`   | (uses mainnet keypair for `cargo test` loop)      |

### After-deploy checks

```sh
# Inspect on-chain state
npm --prefix scripts run cli -- \
  --cluster mainnet-beta \
  --keypair keys/admin-wallet.json \
  --program-id 94Ja6Y8AuzmZHjQiyk2SzvoysnBr3F17nfHGrHm1idAZ \
  status
```

### Optional: rotate upgrade authority to a multisig

Once comfortable, move upgrade authority off the hot admin wallet:

```sh
solana program set-upgrade-authority \
  94Ja6Y8AuzmZHjQiyk2SzvoysnBr3F17nfHGrHm1idAZ \
  --keypair keys/admin-wallet.json \
  --new-upgrade-authority <MULTISIG_PUBKEY> \
  --url mainnet-beta
```

Copy `web/.env.local.example` to `web/.env.local` and flip the `NEXT_PUBLIC_*`
values to your target cluster.

## Security notes

- The program uses `#[account(has_one = admin)]` on every privileged
  instruction and rejects with `Unauthorized` on mismatch.
- Vaults are PDA-owned; only this program can move them out via the
  vault-auth PDA.
- `deposit_rewards` requires `total_staked > 0` (otherwise rewards would be
  silently discarded).
- `propose_admin` does not activate the change; the new admin must sign
  `accept_admin`. This avoids bricking the program by rotating to a key you
  don't control.
- All arithmetic uses `checked_*` / `u128` where an overflow window exists.
