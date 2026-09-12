use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::state::{PendingRotation, TrustedApprovers, Vault, ROTATION_TIMELOCK_SECS};

pub fn request_owner_rotation(ctx: Context<RequestOwnerRotation>, new_owner: Pubkey, _nonce: u64) -> Result<()> {
    let signer_key = ctx.accounts.signer.key();
    let approvers = &ctx.accounts.approvers;
    let mut is_approver = false;
    for i in 0..(approvers.count as usize) {
        if approvers.entries[i] == signer_key {
            is_approver = true;
            break;
        }
    }
    require!(is_approver, VaultError::NotTrustedApprover);

    let now = Clock::get()?.unix_timestamp;
    let executes_at = now
        .checked_add(ROTATION_TIMELOCK_SECS)
        .ok_or(VaultError::AmountOverflow)?;

    let rotation = &mut ctx.accounts.rotation;
    rotation.vault = ctx.accounts.vault.key();
    rotation.new_owner = new_owner;
    rotation.requested_by = signer_key;
    rotation.requested_at = now;
    rotation.executes_at = executes_at;
    rotation.cancelled = false;
    rotation.bump = ctx.bumps.rotation;

    Ok(())
}

#[derive(Accounts)]
#[instruction(new_owner: Pubkey, nonce: u64)]
pub struct RequestOwnerRotation<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    pub vault: Account<'info, Vault>,

    #[account(has_one = vault)]
    pub approvers: Account<'info, TrustedApprovers>,

    #[account(
        init,
        payer = signer,
        space = PendingRotation::SPACE,
        seeds = [b"rotation", vault.key().as_ref(), &nonce.to_le_bytes()],
        bump
    )]
    pub rotation: Account<'info, PendingRotation>,

    pub system_program: Program<'info, System>,
}
