use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::state::{TrustedApprovers, Vault, MAX_APPROVERS};

pub fn add_trusted_approver(ctx: Context<AddTrustedApprover>, address: Pubkey) -> Result<()> {
    require!(
        address != Pubkey::default(),
        VaultError::InvalidApproverAddress
    );
    require!(
        address != ctx.accounts.vault.owner,
        VaultError::CannotApproveOwnWithdrawals
    );

    let approvers = &mut ctx.accounts.approvers;
    require!(
        (approvers.count as usize) < MAX_APPROVERS,
        VaultError::ApproverListFull
    );

    for i in 0..(approvers.count as usize) {
        require!(
            approvers.entries[i] != address,
            VaultError::ApproverAlreadyExists
        );
    }

    // Anti-takeover control: on a personal vault, once at least one approver
    // already exists, adding another approver requires that an EXISTING
    // approver also co-sign — not just the owner alone. This defends against
    // a single compromised owner-session unilaterally escalating to full
    // vault takeover (add own approver -> request rotation to self -> wait
    // out the timelock -> execute). Pool vaults, and the very first approver
    // ever added, are unaffected: the owner-only check remains sufficient.
    if !ctx.accounts.vault.is_pool && approvers.count > 0 {
        let existing_signer = ctx
            .accounts
            .existing_approver
            .as_ref()
            .ok_or(VaultError::ExistingApproverRequired)?;
        let existing_key = existing_signer.key();
        let mut is_existing_approver = false;
        for i in 0..(approvers.count as usize) {
            if approvers.entries[i] == existing_key {
                is_existing_approver = true;
                break;
            }
        }
        require!(is_existing_approver, VaultError::ExistingApproverRequired);
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

    /// Required (must co-sign) when adding a second-or-later approver to a
    /// personal vault; otherwise may be omitted (`None`).
    pub existing_approver: Option<Signer<'info>>,
}
