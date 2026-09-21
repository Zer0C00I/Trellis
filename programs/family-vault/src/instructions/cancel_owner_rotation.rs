use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::state::{PendingRotation, TrustedApprovers, Vault};

pub fn cancel_owner_rotation(ctx: Context<CancelOwnerRotation>) -> Result<()> {
    require!(
        !ctx.accounts.rotation.cancelled,
        VaultError::RotationAlreadyCancelled
    );

    let signer_key = ctx.accounts.signer.key();
    let vault = &ctx.accounts.vault;
    let is_owner = signer_key == vault.owner;

    let approvers = &ctx.accounts.approvers;
    let mut is_approver = false;
    for i in 0..(approvers.count as usize) {
        if approvers.entries[i] == signer_key {
            is_approver = true;
            break;
        }
    }

    require!(is_owner || is_approver, VaultError::NotAuthorizedToCancel);

    ctx.accounts.rotation.cancelled = true;
    Ok(())
}

#[derive(Accounts)]
pub struct CancelOwnerRotation<'info> {
    pub signer: Signer<'info>,

    pub vault: Account<'info, Vault>,

    #[account(has_one = vault)]
    pub approvers: Account<'info, TrustedApprovers>,

    #[account(mut, has_one = vault)]
    pub rotation: Account<'info, PendingRotation>,
}
