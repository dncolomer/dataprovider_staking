use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::{CONFIG_SEED, POOL_SEED, USER_SEED, VAULT_AUTH_SEED};
use crate::error::ErrorCode;
use crate::state::{GlobalConfig, TokenPool, UserStake};

/// Claim all pending USDC rewards from a pool.
///
/// 1. Settle: add any newly accrued rewards (since last touch) to
///    `pending_rewards`.
/// 2. Transfer the full `pending_rewards` amount from reward vault to user
///    USDC ATA.
/// 3. Zero out `pending_rewards` and update counters.
#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    pub user: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
    )]
    pub config: Box<Account<'info, GlobalConfig>>,

    pub stake_mint: Box<Account<'info, Mint>>,

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
    )]
    pub reward_vault: Box<Account<'info, TokenAccount>>,

    #[account(address = config.usdc_mint @ ErrorCode::InvalidRewardMint)]
    pub usdc_mint: Box<Account<'info, Mint>>,

    /// CHECK: vault PDA authority, validated via seeds.
    #[account(
        seeds = [VAULT_AUTH_SEED, stake_mint.key().as_ref()],
        bump = pool.vault_authority_bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [USER_SEED, stake_mint.key().as_ref(), user.key().as_ref()],
        bump = user_stake.bump,
        constraint = user_stake.owner == user.key() @ ErrorCode::Unauthorized,
    )]
    pub user_stake: Box<Account<'info, UserStake>>,

    #[account(
        mut,
        token::mint = usdc_mint,
        token::authority = user,
    )]
    pub user_usdc_account: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<ClaimRewards>) -> Result<()> {
    let pool_acc = ctx.accounts.pool.acc_reward_per_share;
    let user_stake = &mut ctx.accounts.user_stake;

    // Fold any newly earned rewards into `pending_rewards`.
    user_stake.settle(pool_acc)?;

    let to_claim = user_stake.pending_rewards;
    require!(to_claim > 0, ErrorCode::NothingToClaim);

    // Transfer USDC from reward vault to user's USDC account, signed by PDA.
    let stake_mint_key = ctx.accounts.stake_mint.key();
    let vault_auth_seeds: &[&[u8]] = &[
        VAULT_AUTH_SEED,
        stake_mint_key.as_ref(),
        &[ctx.accounts.pool.vault_authority_bump],
    ];
    let signer_seeds: &[&[&[u8]]] = &[vault_auth_seeds];
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        Transfer {
            from: ctx.accounts.reward_vault.to_account_info(),
            to: ctx.accounts.user_usdc_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        },
        signer_seeds,
    );
    token::transfer(cpi_ctx, to_claim)?;

    user_stake.pending_rewards = 0;
    user_stake.total_claimed = user_stake
        .total_claimed
        .checked_add(to_claim)
        .ok_or(ErrorCode::MathOverflow)?;

    let pool = &mut ctx.accounts.pool;
    pool.total_rewards_claimed = pool
        .total_rewards_claimed
        .checked_add(to_claim)
        .ok_or(ErrorCode::MathOverflow)?;

    Ok(())
}
