use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Arithmetic overflow.")]
    MathOverflow,
    #[msg("Caller is not the program admin.")]
    Unauthorized,
    #[msg("Caller is not the pending admin for this handover.")]
    NotPendingAdmin,
    #[msg("No pending admin handover in progress.")]
    NoPendingAdmin,
    #[msg("Maximum number of pools already created.")]
    MaxPoolsReached,
    #[msg("Pool already exists for this mint.")]
    PoolAlreadyExists,
    #[msg("Amount must be greater than zero.")]
    ZeroAmount,
    #[msg("Insufficient staked balance for requested withdrawal.")]
    InsufficientStake,
    #[msg("No rewards available to claim.")]
    NothingToClaim,
    #[msg("Pool has no stakers yet; rewards cannot be distributed.")]
    NoStakersInPool,
    #[msg("Reward vault mint does not match the USDC mint configured in GlobalConfig.")]
    InvalidRewardMint,
    #[msg("Stake vault mint does not match the pool's stake mint.")]
    InvalidStakeMint,
    #[msg("Reward deposit too small to distribute; would round to zero.")]
    RewardDepositTooSmall,
}
