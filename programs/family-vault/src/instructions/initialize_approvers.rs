use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::state::{TrustedApprovers, Vault};

pub fn initialize_approvers(ctx: Context<InitializeApprovers>) -> Result<()> {
    let approvers = &mut ctx.accounts.approvers;
    approvers.vault = ctx.accounts.vault.key();
    approvers.count = 0;
    Ok(())
}

#[derive(Accounts)]
pub struct InitializeApprovers<'info> {
    #[account(mut, constraint = payer.key() == vault.owner @ VaultError::NotVaultOwner)]
    pub payer: Signer<'info>,

    pub vault: Account<'info, Vault>,

    #[account(
        init,
        payer = payer,
        space = TrustedApprovers::SPACE,
        seeds = [b"approvers", vault.key().as_ref()],
        bump
    )]
    pub approvers: Account<'info, TrustedApprovers>,

    pub system_program: Program<'info, System>,
}
