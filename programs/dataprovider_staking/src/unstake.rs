use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::{POOL_SEED, USER_SEED, VAULT_AUTH_SEED};
use crate::error::ErrorCode;
use crate::state::{TokenPool, UserStake, ACC_PRECISION};

/// Unstake tokens from the pool and return them to the user. Settles any
/// outstanding rewards into `pending_rewards` first (claim is a separate
/// instruction so unstake can happen even if the reward vault is temporarily
/// empty during a bookkeeping window).
#[derive(Accounts)]
pub struct Unstake<'info> {
    pub user: Signer<'info>,

    pub stake_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        seeds = [POOL_SEED, stake_mint.key().as_ref()],
        bump = pool.bump,
        has_one = stake_mint @ ErrorCode::InvalidStakeMint,
        has_one = stake_vault @ ErrorCode::InvalidStakeMint,
    )]
    pub pool: Box<Account<'info, TokenPool>>,

    #[account(
        mut,
        token::mint = stake_mint,
    )]
    pub stake_vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: PDA authority for vaults, validated via seeds.
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
        token::mint = stake_mint,
        token::authority = user,
    )]
    pub user_token_account: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Unstake>, amount: u64) -> Result<()> {
    require!(amount > 0, ErrorCode::ZeroAmount);
    require!(
        ctx.accounts.user_stake.amount >= amount,
        ErrorCode::InsufficientStake
    );

    let pool_acc = ctx.accounts.pool.acc_reward_per_share;
    let user_stake = &mut ctx.accounts.user_stake;

    // Settle before mutating amount.
    user_stake.settle(pool_acc)?;

    user_stake.amount = user_stake
        .amount
        .checked_sub(amount)
        .ok_or(ErrorCode::MathOverflow)?;
    user_stake.reward_debt = (user_stake.amount as u128)
        .checked_mul(pool_acc)
        .ok_or(ErrorCode::MathOverflow)?
        / ACC_PRECISION;

    // CPI: transfer from vault back to user, signed by vault PDA.
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
            from: ctx.accounts.stake_vault.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        },
        signer_seeds,
    );
    token::transfer(cpi_ctx, amount)?;

    let pool = &mut ctx.accounts.pool;
    pool.total_staked = pool
        .total_staked
        .checked_sub(amount)
        .ok_or(ErrorCode::MathOverflow)?;

    Ok(())
}
