import { PublicKey } from "@solana/web3.js";

/**
 * Deployed program id. Kept in sync with `declare_id!` in the Rust program.
 * This is a placeholder for local testing; update before devnet/mainnet deploy.
 */
export const DATAPROVIDER_STAKING_PROGRAM_ID = new PublicKey(
  "GyZKxaZaLZtKLes5JgJfEHZUBhrtuMgTNtuLiEU59Bqd",
);

/**
 * Intended production admin authority (per operator directive). Used only as a
 * constant reference for admin scripts / UI display; the on-chain admin is
 * whatever `GlobalConfig.admin` says at any given time.
 */
export const PRODUCTION_ADMIN = new PublicKey(
  "6HGeNL5852ykqQNiwT6sC5YFu1xBBwvgtVnUWuf5EfEP",
);

/**
 * The first pool mint: $GHC1CHEM.
 */
export const GHC1CHEM_MINT = new PublicKey(
  "3pi9trvC6hrMUHHhQnQy5aAPk5CzxAGxsLyiXzshpump",
);

/**
 * Canonical USDC mainnet mint. On localnet/devnet the admin scripts mint a
 * mock USDC mint whose pubkey is written into `GlobalConfig.usdc_mint`.
 */
export const USDC_MAINNET_MINT = new PublicKey(
  "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
);

/**
 * Precision factor applied to the `acc_reward_per_share` accumulator on-chain.
 * Keep in sync with `ACC_PRECISION` in `state.rs`.
 */
export const ACC_PRECISION = 1_000_000_000_000n;

/**
 * Hard cap on pools; must match MAX_POOLS on-chain.
 */
export const MAX_POOLS = 5;
