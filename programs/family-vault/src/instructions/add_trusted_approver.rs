use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::state::{TrustedApprovers, Vault, MAX_APPROVERS};

pub fn add_trusted_approver(ctx: Context<AddTrustedApprover>, address: Pubkey) -> Result<()> {
    let approvers = &mut ctx.accounts.approvers;
    require!((approvers.count as usize) < MAX_APPROVERS, VaultError::ApproverListFull);

    for i in 0..(approvers.count as usize) {
        require!(approvers.entries[i] != address, VaultError::ApproverAlreadyExists);
    }

    let idx = approvers.count as usize;
    approvers.entries[idx] = address;
    approvers.count += 1;

    Ok(())
}

#[derive(Accounts)]
pub struct AddTrustedApprover<'info> {
    #[account(constraint = signer.key() == vault.owner @ VaultError::NotVaultOwner)]
    pub signer: Signer<'info>,

    pub vault: Account<'info, Vault>,

    #[account(mut, has_one = vault)]
    pub approvers: Account<'info, TrustedApprovers>,
}
