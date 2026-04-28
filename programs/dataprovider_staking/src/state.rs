use anchor_lang::prelude::*;

/// Scalar used for fixed-point accumulator math: acc_reward_per_share is scaled by 1e12.
pub const ACC_PRECISION: u128 = 1_000_000_000_000;

/// Hard cap on number of token pools the protocol can ever host.
pub const MAX_POOLS: u8 = 5;

/// Singleton config account. Holds the admin authority (updateable only by the
/// current admin) and the USDC mint used for every pool's reward vault.
#[account]
#[derive(InitSpace)]
pub struct GlobalConfig {
    /// Authority permitted to add pools, deposit rewards, and rotate the admin.
    pub admin: Pubkey,
    /// Pending admin for 2-step admin rotation. `Pubkey::default()` when no
    /// handover is in progress.
    pub pending_admin: Pubkey,
    /// USDC mint used as the reward currency for every pool.
    pub usdc_mint: Pubkey,
    /// Number of pools that have been created so far.
    pub pool_count: u8,
    /// PDA bump.
    pub bump: u8,
}

/// Per-stake-mint pool. Holds the staking vault, reward (USDC) vault, and the
/// running pro-rata accumulator used to compute each user's earnings.
#[account]
#[derive(InitSpace)]
pub struct TokenPool {
    /// The SPL mint users stake into this pool.
    pub stake_mint: Pubkey,
    /// PDA-owned token account holding all staked tokens for this pool.
    pub stake_vault: Pubkey,
    /// PDA-owned token account holding all undistributed USDC rewards.
    pub reward_vault: Pubkey,
    /// Total tokens currently staked across all users in this pool.
    pub total_staked: u64,
    /// Accumulated USDC reward per staked token unit, scaled by `ACC_PRECISION`.
    ///
    /// Invariant: increases monotonically every time rewards are deposited
    /// while `total_staked > 0`.
    pub acc_reward_per_share: u128,
    /// Lifetime USDC deposited as rewards into this pool (for accounting).
    pub total_rewards_deposited: u64,
    /// Lifetime USDC actually claimed by users from this pool.
    pub total_rewards_claimed: u64,
    /// PDA bump for the pool account.
    pub bump: u8,
    /// PDA bump for the stake vault authority.
    pub vault_authority_bump: u8,
}

/// Per-user, per-pool staking position.
#[account]
#[derive(InitSpace)]
pub struct UserStake {
    /// Owner of this stake position.
    pub owner: Pubkey,
    /// Pool this position belongs to (redundant but useful for indexers).
    pub stake_mint: Pubkey,
    /// Tokens currently staked by the owner in this pool.
    pub amount: u64,
    /// Reward-debt in MasterChef accounting. Equals `amount * acc_reward_per_share / ACC_PRECISION`
    /// at the time of the last settlement.
    pub reward_debt: u128,
    /// Rewards already settled to the user's "claimable" bucket but not yet
    /// transferred out of the reward vault.
    pub pending_rewards: u64,
    /// Lifetime USDC claimed by this user from this pool.
    pub total_claimed: u64,
    /// PDA bump.
    pub bump: u8,
}

impl UserStake {
    /// Compute the incremental earnings since the last settlement and fold
    /// them into `pending_rewards`, then sync the reward-debt to the pool's
    /// current accumulator.
    ///
    /// Safe to call whenever the account is freshly initialized (amount=0,
    /// reward_debt=0) -- it simply becomes a no-op except for syncing debt.
    pub fn settle(&mut self, pool_acc_reward_per_share: u128) -> Result<()> {
        if self.amount > 0 {
            // earned = amount * acc / PRECISION - reward_debt
            let accrued: u128 = (self.amount as u128)
                .checked_mul(pool_acc_reward_per_share)
                .ok_or(crate::error::ErrorCode::MathOverflow)?
                / ACC_PRECISION;
            let delta = accrued
                .checked_sub(self.reward_debt)
                .ok_or(crate::error::ErrorCode::MathOverflow)?;
            // delta bounded by total rewards deposited, which is u64.
            let delta_u64: u64 = delta
                .try_into()
                .map_err(|_| crate::error::ErrorCode::MathOverflow)?;
            self.pending_rewards = self
                .pending_rewards
                .checked_add(delta_u64)
                .ok_or(crate::error::ErrorCode::MathOverflow)?;
        }
        // Sync reward-debt to "zero-out" earnings for the new amount.
        self.reward_debt = (self.amount as u128)
            .checked_mul(pool_acc_reward_per_share)
            .ok_or(crate::error::ErrorCode::MathOverflow)?
            / ACC_PRECISION;
        Ok(())
    }
}
