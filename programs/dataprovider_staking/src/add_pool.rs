use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::{CONFIG_SEED, POOL_SEED, VAULT_AUTH_SEED};
use crate::error::ErrorCode;
use crate::state::{GlobalConfig, TokenPool, MAX_POOLS};

/// Admin-only: register a new stake mint and create its stake + reward vaults.
///
/// Each pool has:
///   - a `stake_vault` (a TokenAccount for `stake_mint`) owned by the pool's
///     `vault_authority` PDA. Uses the same token program as the stake mint
///     (classic SPL or Token-2022), so Token-2022 mints like $GHC1CHEM are
///     supported natively.
///   - a `reward_vault` (a TokenAccount for `usdc_mint`) also owned by the
///     same `vault_authority` PDA, funded by the admin and claimed by users.
///
/// The `vault_authority` is derived as [VAULT_AUTH_SEED, stake_mint] and never
/// holds non-token state directly. Ownership by a PDA means only this program
/// (via CPI with the correct seeds) can move funds out of the vaults.
///
/// IMPORTANT: the caller MUST pass the token program matching `stake_mint`'s
/// owner. Anchor's `InterfaceAccount<Mint>` accepts either SPL Token or
/// Token-2022, and `Interface<TokenInterface>` resolves the matching program
/// at CPI time. We do the same for USDC, but since USDC has a single known
/// program today, we put it on the same interface for consistency.
#[derive(Accounts)]
pub struct AddPool<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = admin @ ErrorCode::Unauthorized,
        constraint = (config.pool_count as u8) < MAX_POOLS @ ErrorCode::MaxPoolsReached,
    )]
    pub config: Account<'info, GlobalConfig>,

    pub admin: Signer<'info>,

    pub stake_mint: Box<InterfaceAccount<'info, Mint>>,

    /// USDC mint; must match the one recorded at `initialize`.
    #[account(address = config.usdc_mint @ ErrorCode::InvalidRewardMint)]
    pub usdc_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init,
        payer = payer,
        space = 8 + TokenPool::INIT_SPACE,
        seeds = [POOL_SEED, stake_mint.key().as_ref()],
        bump,
    )]
    pub pool: Box<Account<'info, TokenPool>>,

    /// PDA authority that will own both vaults. Not deserialized; only used
    /// to derive its bump.
    /// CHECK: PDA derived with seeds [VAULT_AUTH_SEED, stake_mint]; no data.
    #[account(
        seeds = [VAULT_AUTH_SEED, stake_mint.key().as_ref()],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        token::mint = stake_mint,
        token::authority = vault_authority,
        token::token_program = stake_token_program,
        seeds = [b"stake_vault", stake_mint.key().as_ref()],
        bump,
    )]
    pub stake_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init,
        payer = payer,
        token::mint = usdc_mint,
        token::authority = vault_authority,
        token::token_program = usdc_token_program,
        seeds = [b"reward_vault", stake_mint.key().as_ref()],
        bump,
    )]
    pub reward_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Token program matching the `stake_mint` owner (SPL Token or Token-2022).
    pub stake_token_program: Interface<'info, TokenInterface>,
    /// Token program matching the `usdc_mint` owner (SPL Token or Token-2022).
    /// Usually classic SPL Token for mainnet USDC.
    pub usdc_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<AddPool>) -> Result<()> {
    let pool = &mut ctx.accounts.pool;
    pool.stake_mint = ctx.accounts.stake_mint.key();
    pool.stake_vault = ctx.accounts.stake_vault.key();
    pool.reward_vault = ctx.accounts.reward_vault.key();
    pool.total_staked = 0;
    pool.acc_reward_per_share = 0;
    pool.total_rewards_deposited = 0;
    pool.total_rewards_claimed = 0;
    pool.bump = ctx.bumps.pool;
    pool.vault_authority_bump = ctx.bumps.vault_authority;

    let config = &mut ctx.accounts.config;
    config.pool_count = config
        .pool_count
        .checked_add(1)
        .ok_or(ErrorCode::MaxPoolsReached)?;
    Ok(())
}
