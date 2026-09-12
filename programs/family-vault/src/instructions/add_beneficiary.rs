use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::state::{Beneficiary, BeneficiaryList, Vault, BENEFICIARY_COOLDOWN_SECS, BENEFICIARY_NAME_LEN, MAX_BENEFICIARIES};

pub fn add_beneficiary(ctx: Context<AddBeneficiary>, name: String, address: Pubkey) -> Result<()> {
    require!(name.len() <= BENEFICIARY_NAME_LEN, VaultError::NameTooLong);

    let list = &mut ctx.accounts.beneficiary_list;
    require!((list.count as usize) < MAX_BENEFICIARIES, VaultError::BeneficiaryListFull);

    for i in 0..(list.count as usize) {
        require!(list.entries[i].address != address, VaultError::BeneficiaryAlreadyExists);
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
        active_after: now.checked_add(BENEFICIARY_COOLDOWN_SECS).ok_or(VaultError::AmountOverflow)?,
    };
    list.count += 1;

    Ok(())
}

#[derive(Accounts)]
pub struct AddBeneficiary<'info> {
    #[account(constraint = signer.key() == vault.owner @ VaultError::NotVaultOwner)]
    pub signer: Signer<'info>,

    pub vault: Account<'info, Vault>,

    #[account(mut, has_one = vault)]
    pub beneficiary_list: Account<'info, BeneficiaryList>,
}
