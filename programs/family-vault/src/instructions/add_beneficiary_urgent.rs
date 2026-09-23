use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::state::{
    Beneficiary, BeneficiaryList, TrustedApprovers, Vault, BENEFICIARY_NAME_LEN, MAX_BENEFICIARIES,
};

/// Two-of-approvers fast lane for adding a beneficiary without the normal
/// 48h cooldown (see `add_beneficiary`). The owner alone can never trigger
/// this — it requires two DISTINCT signers who are both already on the
/// vault's `TrustedApprovers` list. This exists because the ordinary
/// cooldown protects against a single compromised owner-key session, but
/// also blocks a genuinely urgent, legitimate add (e.g. a new caregiver
/// who needs to be paid today) with no way through. Requiring two
/// independent trusted people to agree raises the cost of abusing this
/// path to "compromise or socially engineer two people at once", instead
/// of removing the protection outright.
pub fn add_beneficiary_urgent(
    ctx: Context<AddBeneficiaryUrgent>,
    name: String,
    address: Pubkey,
) -> Result<()> {
    require!(name.len() <= BENEFICIARY_NAME_LEN, VaultError::NameTooLong);

    let approver_one = ctx.accounts.approver_one.key();
    let approver_two = ctx.accounts.approver_two.key();
    require!(approver_one != approver_two, VaultError::DuplicateApprover);

    let approvers = &ctx.accounts.approvers;
    let mut one_is_approver = false;
    let mut two_is_approver = false;
    for i in 0..(approvers.count as usize) {
        if approvers.entries[i] == approver_one {
            one_is_approver = true;
        }
        if approvers.entries[i] == approver_two {
            two_is_approver = true;
        }
    }
    require!(one_is_approver, VaultError::NotTrustedApprover);
    require!(two_is_approver, VaultError::NotTrustedApprover);

    let list = &mut ctx.accounts.beneficiary_list;
    require!(
        (list.count as usize) < MAX_BENEFICIARIES,
        VaultError::BeneficiaryListFull
    );
    for i in 0..(list.count as usize) {
        require!(
            list.entries[i].address != address,
            VaultError::BeneficiaryAlreadyExists
        );
    }

    let now = Clock::get()?.unix_timestamp;
    let mut name_bytes = [0u8; BENEFICIARY_NAME_LEN];
    let src = name.as_bytes();
    name_bytes[..src.len()].copy_from_slice(src);

    let idx = list.count as usize;
    list.entries[idx] = Beneficiary {
        address,
        name: name_bytes,
        name_len: src.len() as u8,
        added_at: now,
        // Immediately active — this is the entire point of the two-approver
        // override, unlike ordinary `add_beneficiary`'s `now + 48h`.
        active_after: now,
    };
    list.count += 1;

    Ok(())
}

#[derive(Accounts)]
pub struct AddBeneficiaryUrgent<'info> {
    pub approver_one: Signer<'info>,
    pub approver_two: Signer<'info>,

    pub vault: Account<'info, Vault>,

    #[account(has_one = vault)]
    pub approvers: Account<'info, TrustedApprovers>,

    #[account(mut, has_one = vault)]
    pub beneficiary_list: Account<'info, BeneficiaryList>,
}
