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

You should see **15/15 tests green**: `test_initialize` (4) and
`test_staking_flow` (11), covering:

- initialize sets admin / usdc mint
- 2-step admin rotation (propose / accept / cancel / interloper rejected)
- only admin can `add_pool`
- stake, partial unstake, full unstake return principal
- deposits fail when pool has zero stakers
- claim fails when pending == 0
- single-staker earns 100% of rewards
- two stakers split rewards 1:3
- late stakers only earn on *future* deposits
- unstaking preserves pending rewards for later claim
- one user staking in multiple pools earns independently per pool
- adding stake after a deposit does NOT backfill past rewards

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

## Deploying to devnet (when you're ready)

```sh
# generate a fresh program keypair (once)
solana-keygen new -o target/deploy/dataprovider_staking-keypair.json
# update declare_id! in programs/dataprovider_staking/src/lib.rs
# update [programs.devnet] in Anchor.toml
anchor build
anchor deploy --provider.cluster devnet
# then rotate admin immediately to production admin:
dps-admin --cluster devnet initialize --usdc-mint <USDC_DEVNET>
dps-admin --cluster devnet propose-admin --new-admin 6HGeNL5852ykqQNiwT6sC5YFu1xBBwvgtVnUWuf5EfEP
# (sign accept-admin from the production admin wallet)
```

Copy `web/.env.local.example` to `web/.env.local` and flip the NEXT_PUBLIC_*
values to devnet.

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
