use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    self, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::constants::{POOL_SEED, USER_SEED};
use crate::error::ErrorCode;
use crate::state::{TokenPool, UserStake};

/// Stake tokens of `pool.stake_mint` into the pool. If this is the user's
/// first stake into this pool, the `UserStake` PDA is initialized.
///
/// Settlement order (critical): we first settle pending rewards against the
/// user's *current* stake using the pool's existing accumulator, then
/// increment `amount`, then resync `reward_debt`. This guarantees a freshly
/// added amount earns only from rewards deposited *after* this tx.
///
/// Uses `transfer_checked` so the transfer is valid for both classic SPL
/// Token and Token-2022 mints (Token-2022 requires the mint + decimals to
/// be passed explicitly).
#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub stake_mint: Box<InterfaceAccount<'info, Mint>>,

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
        token::token_program = token_program,
    )]
    pub stake_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = user,
        space = 8 + UserStake::INIT_SPACE,
        seeds = [USER_SEED, stake_mint.key().as_ref(), user.key().as_ref()],
        bump,
    )]
    pub user_stake: Box<Account<'info, UserStake>>,

    #[account(
        mut,
        token::mint = stake_mint,
        token::authority = user,
        token::token_program = token_program,
    )]
    pub user_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<Stake>, amount: u64) -> Result<()> {
    require!(amount > 0, ErrorCode::ZeroAmount);

    let pool_acc = ctx.accounts.pool.acc_reward_per_share;
    let user_stake = &mut ctx.accounts.user_stake;

    // Initialize fields on first-time stake.
    if user_stake.owner == Pubkey::default() {
        user_stake.owner = ctx.accounts.user.key();
        user_stake.stake_mint = ctx.accounts.stake_mint.key();
        user_stake.amount = 0;
        user_stake.reward_debt = 0;
        user_stake.pending_rewards = 0;
        user_stake.total_claimed = 0;
        user_stake.bump = ctx.bumps.user_stake;
    }

    // Settle rewards earned so far based on the CURRENT amount.
    user_stake.settle(pool_acc)?;

    // Transfer tokens from user to vault (transfer_checked works for both
    // classic SPL and Token-2022).
    let decimals = ctx.accounts.stake_mint.decimals;
    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.key(),
        TransferChecked {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.stake_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
            mint: ctx.accounts.stake_mint.to_account_info(),
        },
    );
    token_interface::transfer_checked(cpi_ctx, amount, decimals)?;

    // Update balances.
    user_stake.amount = user_stake
        .amount
        .checked_add(amount)
        .ok_or(ErrorCode::MathOverflow)?;
    // Re-sync reward-debt to the NEW amount so future settlements are correct.
    user_stake.reward_debt = (user_stake.amount as u128)
        .checked_mul(pool_acc)
        .ok_or(ErrorCode::MathOverflow)?
        / crate::state::ACC_PRECISION;

    let pool = &mut ctx.accounts.pool;
    pool.total_staked = pool
        .total_staked
        .checked_add(amount)
        .ok_or(ErrorCode::MathOverflow)?;

    Ok(())
}
