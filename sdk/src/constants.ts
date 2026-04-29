import { PublicKey } from "@solana/web3.js";

/**
 * Deployed program id. Kept in sync with `declare_id!` in the Rust program.
 *
 * Mainnet / localnet use the same id (`94Ja6Y8A...`); devnet still uses the
 * legacy `AnConH6PV...` id. Callers targeting devnet should override via the
 * `programId` option on {@link StakingClient}.
 */
export const DATAPROVIDER_STAKING_PROGRAM_ID = new PublicKey(
  "94Ja6Y8AuzmZHjQiyk2SzvoysnBr3F17nfHGrHm1idAZ",
);

/** Legacy devnet program id (for existing devnet state). */
export const DATAPROVIDER_STAKING_PROGRAM_ID_DEVNET = new PublicKey(
  "AnConH6PVX1UQYtdPgAgUNMowphcragEjbGsx3nQJ6up",
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
