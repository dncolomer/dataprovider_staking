use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    self, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::constants::{CONFIG_SEED, POOL_SEED};
use crate::error::ErrorCode;
use crate::state::{GlobalConfig, TokenPool, ACC_PRECISION};

/// Admin-only: deposit USDC rewards into a specific pool. The deposit amount
/// is split pro-rata across all currently-staked users by incrementing
/// `acc_reward_per_share` by `amount * ACC_PRECISION / total_staked`.
///
/// Requires `total_staked > 0`. If there are no stakers, the admin should
/// wait until at least one user stakes (otherwise those rewards would be
/// "lost" to an empty pool).
///
/// Also rejects deposits so small that `delta` rounds to zero (the USDC
/// would be transferred into the vault but could never be claimed).
#[derive(Accounts)]
pub struct DepositRewards<'info> {
    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = admin @ ErrorCode::Unauthorized,
    )]
    pub config: Box<Account<'info, GlobalConfig>>,

    pub admin: Signer<'info>,

    pub stake_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        seeds = [POOL_SEED, stake_mint.key().as_ref()],
        bump = pool.bump,
        has_one = stake_mint @ ErrorCode::InvalidStakeMint,
        has_one = reward_vault @ ErrorCode::InvalidRewardMint,
    )]
    pub pool: Box<Account<'info, TokenPool>>,

    #[account(
        mut,
        token::mint = usdc_mint,
        token::token_program = token_program,
    )]
    pub reward_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(address = config.usdc_mint @ ErrorCode::InvalidRewardMint)]
    pub usdc_mint: Box<InterfaceAccount<'info, Mint>>,

    /// The admin's USDC source account.
    #[account(
        mut,
        token::mint = usdc_mint,
        token::authority = admin,
        token::token_program = token_program,
    )]
    pub admin_usdc_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Token program matching the USDC mint (classic SPL on mainnet today).
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handler(ctx: Context<DepositRewards>, amount: u64) -> Result<()> {
    require!(amount > 0, ErrorCode::ZeroAmount);
    require!(
        ctx.accounts.pool.total_staked > 0,
        ErrorCode::NoStakersInPool
    );

    // Transfer USDC from admin to reward vault.
    let decimals = ctx.accounts.usdc_mint.decimals;
    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.key(),
        TransferChecked {
            from: ctx.accounts.admin_usdc_account.to_account_info(),
            to: ctx.accounts.reward_vault.to_account_info(),
            authority: ctx.accounts.admin.to_account_info(),
            mint: ctx.accounts.usdc_mint.to_account_info(),
        },
    );
    token_interface::transfer_checked(cpi_ctx, amount, decimals)?;

    // Update accumulator: rewards-per-share += amount * PRECISION / total_staked.
    let pool = &mut ctx.accounts.pool;
    let delta: u128 = (amount as u128)
        .checked_mul(ACC_PRECISION)
        .ok_or(ErrorCode::MathOverflow)?
        / (pool.total_staked as u128);
    require!(delta > 0, ErrorCode::RewardDepositTooSmall);
    pool.acc_reward_per_share = pool
        .acc_reward_per_share
        .checked_add(delta)
        .ok_or(ErrorCode::MathOverflow)?;
    pool.total_rewards_deposited = pool
        .total_rewards_deposited
        .checked_add(amount)
        .ok_or(ErrorCode::MathOverflow)?;

    Ok(())
}
