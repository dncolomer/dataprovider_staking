use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

use crate::constants::CONFIG_SEED;
use crate::state::GlobalConfig;

/// One-time initialization of the program. Creates the `GlobalConfig` singleton,
/// records the admin authority and the USDC mint used for all reward pools.
///
/// The signer becomes the initial admin. This is intentional: whoever bootstraps
/// the program on-chain is the owner. After deploy, the admin can be rotated
/// to the production owner (e.g. `6HGeNL5852ykqQNiwT6sC5YFu1xBBwvgtVnUWuf5EfEP`)
/// via `update_admin` -> `accept_admin`.
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The initial admin. Must sign so that the admin key is authenticated at
    /// bootstrap (prevents front-running of config creation with a wrong admin).
    pub admin: Signer<'info>,

    /// The USDC mint used as the reward currency for every pool.
    pub usdc_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = payer,
        space = 8 + GlobalConfig::INIT_SPACE,
        seeds = [CONFIG_SEED],
        bump,
    )]
    pub config: Account<'info, GlobalConfig>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Initialize>) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.admin = ctx.accounts.admin.key();
    config.pending_admin = Pubkey::default();
    config.usdc_mint = ctx.accounts.usdc_mint.key();
    config.pool_count = 0;
    config.bump = ctx.bumps.config;
    Ok(())
}
