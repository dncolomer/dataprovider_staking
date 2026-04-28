use anchor_lang::prelude::*;

/// PDA seed for `GlobalConfig`.
#[constant]
pub const CONFIG_SEED: &[u8] = b"config";

/// PDA seed prefix for a `TokenPool`. Full seeds: [`POOL_SEED`, stake_mint].
#[constant]
pub const POOL_SEED: &[u8] = b"pool";

/// PDA seed prefix for a pool's stake-vault authority. Full seeds:
/// [`VAULT_AUTH_SEED`, stake_mint]. This PDA owns both the stake-vault
/// and the reward-vault token accounts for the pool.
#[constant]
pub const VAULT_AUTH_SEED: &[u8] = b"vault_auth";

/// PDA seed prefix for a `UserStake`. Full seeds: [`USER_SEED`, stake_mint, owner].
#[constant]
pub const USER_SEED: &[u8] = b"user";
