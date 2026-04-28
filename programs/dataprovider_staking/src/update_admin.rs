use anchor_lang::prelude::*;

use crate::constants::CONFIG_SEED;
use crate::error::ErrorCode;
use crate::state::GlobalConfig;

/// Two-step admin rotation. Current admin proposes a new admin; the new admin
/// must then call `accept_admin` to activate the change. This prevents
/// accidentally transferring admin to a wrong/unowned key.
#[derive(Accounts)]
pub struct ProposeAdmin<'info> {
    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = admin @ ErrorCode::Unauthorized,
    )]
    pub config: Account<'info, GlobalConfig>,

    pub admin: Signer<'info>,

    /// CHECK: Just a pubkey we're writing into config.pending_admin. The real
    /// authentication happens when this pubkey signs `accept_admin`.
    pub new_admin: UncheckedAccount<'info>,
}

pub fn propose_handler(ctx: Context<ProposeAdmin>) -> Result<()> {
    ctx.accounts.config.pending_admin = ctx.accounts.new_admin.key();
    Ok(())
}

#[derive(Accounts)]
pub struct AcceptAdmin<'info> {
    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump,
    )]
    pub config: Account<'info, GlobalConfig>,

    /// The new admin; must equal `config.pending_admin` and sign.
    pub new_admin: Signer<'info>,
}

pub fn accept_handler(ctx: Context<AcceptAdmin>) -> Result<()> {
    let config = &mut ctx.accounts.config;
    require!(
        config.pending_admin != Pubkey::default(),
        ErrorCode::NoPendingAdmin
    );
    require_keys_eq!(
        config.pending_admin,
        ctx.accounts.new_admin.key(),
        ErrorCode::NotPendingAdmin
    );
    config.admin = ctx.accounts.new_admin.key();
    config.pending_admin = Pubkey::default();
    Ok(())
}

/// Admin may cancel a pending handover without needing the proposed admin to do anything.
#[derive(Accounts)]
pub struct CancelAdminProposal<'info> {
    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = admin @ ErrorCode::Unauthorized,
    )]
    pub config: Account<'info, GlobalConfig>,

    pub admin: Signer<'info>,
}

pub fn cancel_handler(ctx: Context<CancelAdminProposal>) -> Result<()> {
    ctx.accounts.config.pending_admin = Pubkey::default();
    Ok(())
}
