/**
 * PDA derivation helpers for the dataprovider_staking program.
 *
 * All PDAs are derived from byte-string seeds; keep these in lockstep with
 * the `constants.rs` module in the on-chain program.
 */
import { PublicKey } from "@solana/web3.js";

export const CONFIG_SEED = Buffer.from("config");
export const POOL_SEED = Buffer.from("pool");
export const VAULT_AUTH_SEED = Buffer.from("vault_auth");
export const USER_SEED = Buffer.from("user");
export const STAKE_VAULT_SEED = Buffer.from("stake_vault");
export const REWARD_VAULT_SEED = Buffer.from("reward_vault");

export function findConfigPda(programId: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([CONFIG_SEED], programId);
}

export function findPoolPda(
  stakeMint: PublicKey,
  programId: PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [POOL_SEED, stakeMint.toBuffer()],
    programId,
  );
}

export function findVaultAuthorityPda(
  stakeMint: PublicKey,
  programId: PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [VAULT_AUTH_SEED, stakeMint.toBuffer()],
    programId,
  );
}

export function findStakeVaultPda(
  stakeMint: PublicKey,
  programId: PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [STAKE_VAULT_SEED, stakeMint.toBuffer()],
    programId,
  );
}

export function findRewardVaultPda(
  stakeMint: PublicKey,
  programId: PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [REWARD_VAULT_SEED, stakeMint.toBuffer()],
    programId,
  );
}

export function findUserStakePda(
  stakeMint: PublicKey,
  owner: PublicKey,
  programId: PublicKey,
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [USER_SEED, stakeMint.toBuffer(), owner.toBuffer()],
    programId,
  );
}
