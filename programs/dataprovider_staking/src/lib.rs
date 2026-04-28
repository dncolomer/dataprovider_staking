//! # dataprovider_staking
//!
//! Multi-mint staking program with USDC dividend distribution.
//!
//! Users stake tokens from up to 5 different SPL mints (the first being
//! `$GHC1CHEM` / `3pi9trvC6hrMUHHhQnQy5aAPk5CzxAGxsLyiXzshpump`). The protocol
//! admin periodically deposits USDC revenue into each pool; those deposits
//! are distributed pro-rata to that pool's stakers using a classic
//! reward-per-share accumulator (gas O(1) per deposit regardless of staker
//! count). Each pool has an independent USDC reward vault; a user can stake
//! into any subset of pools independently.
//!
//! Admin is a single pubkey stored in `GlobalConfig`, rotated via a two-step
//! propose/accept flow (guards against transferring admin to a mistyped or
//! unowned key). Admin at deploy-time is whoever signs `initialize`; the
//! operator is expected to rotate it to the production admin
//! (`6HGeNL5852ykqQNiwT6sC5YFu1xBBwvgtVnUWuf5EfEP`) immediately after.

use anchor_lang::prelude::*;

pub mod constants;
pub mod error;
pub mod state;

// Instruction submodules re-exported directly at the crate root so Anchor's
// `#[program]` macro can resolve `crate::<snake>::__client_accounts_<snake>`.
pub mod add_pool;
pub mod claim_rewards;
pub mod deposit_rewards;
pub mod initialize;
pub mod stake;
pub mod unstake;
pub mod update_admin;

pub use constants::*;
pub use state::*;
pub use add_pool::AddPool;
pub use claim_rewards::ClaimRewards;
pub use deposit_rewards::DepositRewards;
pub use initialize::Initialize;
pub use stake::Stake;
pub use unstake::Unstake;
pub use update_admin::{AcceptAdmin, CancelAdminProposal, ProposeAdmin};

// Anchor's `#[program]` macro references `crate::__client_accounts_<snake>::*`
// to build the `accounts` client-facing module. Each `#[derive(Accounts)]`
// struct defines a `pub(crate)` module of that name at its own module site,
// so we bring them into crate scope here.
pub(crate) use add_pool::__client_accounts_add_pool;
pub(crate) use claim_rewards::__client_accounts_claim_rewards;
pub(crate) use deposit_rewards::__client_accounts_deposit_rewards;
pub(crate) use initialize::__client_accounts_initialize;
pub(crate) use stake::__client_accounts_stake;
pub(crate) use unstake::__client_accounts_unstake;
pub(crate) use update_admin::{
    __client_accounts_accept_admin, __client_accounts_cancel_admin_proposal,
    __client_accounts_propose_admin,
};

declare_id!("AnConH6PVX1UQYtdPgAgUNMowphcragEjbGsx3nQJ6up");

#[program]
pub mod dataprovider_staking {
    use super::*;

    /// Bootstraps the program. Signer becomes the initial admin.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        crate::initialize::handler(ctx)
    }

    /// Admin: propose a new admin (2-step transfer).
    pub fn propose_admin(ctx: Context<ProposeAdmin>) -> Result<()> {
        crate::update_admin::propose_handler(ctx)
    }

    /// New admin: accept a proposed admin rotation.
    pub fn accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
        crate::update_admin::accept_handler(ctx)
    }

    /// Admin: cancel a pending admin rotation.
    pub fn cancel_admin_proposal(ctx: Context<CancelAdminProposal>) -> Result<()> {
        crate::update_admin::cancel_handler(ctx)
    }

    /// Admin: register a new stake-mint pool (with stake + reward vaults).
    pub fn add_pool(ctx: Context<AddPool>) -> Result<()> {
        crate::add_pool::handler(ctx)
    }

    /// User: deposit `amount` of `stake_mint` into the pool.
    pub fn stake(ctx: Context<Stake>, amount: u64) -> Result<()> {
        crate::stake::handler(ctx, amount)
    }

    /// User: withdraw `amount` of their staked tokens from the pool.
    pub fn unstake(ctx: Context<Unstake>, amount: u64) -> Result<()> {
        crate::unstake::handler(ctx, amount)
    }

    /// Admin: deposit `amount` USDC into a pool's reward vault. Distributed
    /// pro-rata to current stakers via the reward-per-share accumulator.
    pub fn deposit_rewards(ctx: Context<DepositRewards>, amount: u64) -> Result<()> {
        crate::deposit_rewards::handler(ctx, amount)
    }

    /// User: claim all pending USDC rewards from a pool.
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        crate::claim_rewards::handler(ctx)
    }
}
