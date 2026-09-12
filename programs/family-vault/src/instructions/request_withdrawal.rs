use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

use crate::errors::VaultError;
use crate::state::{BeneficiaryList, PendingWithdrawal, Vault};

pub fn request_withdrawal(ctx: Context<RequestWithdrawal>, amount: u64, _nonce: u64) -> Result<()> {
    require!(amount > 0, VaultError::ZeroAmount);

    let vault = &ctx.accounts.vault;
    require!(vault.is_pool, VaultError::NotPoolVault);

    let to_key = ctx.accounts.recipient_token_account.owner;
    let list = &ctx.accounts.beneficiary_list;
    let clock = Clock::get()?;

    let mut found = false;
    for i in 0..(list.count as usize) {
        let b = &list.entries[i];
        if b.address == to_key {
            require!(clock.unix_timestamp >= b.active_after, VaultError::BeneficiaryCooldownActive);
            found = true;
            break;
        }
    }
    require!(found, VaultError::UnknownBeneficiary);

    let request = &mut ctx.accounts.pending_withdrawal;
    request.vault = vault.key();
    request.amount = amount;
    request.to = to_key;
    request.requested_at = clock.unix_timestamp;
    request.approved_by = None;
    request.executed = false;
    request.bump = ctx.bumps.pending_withdrawal;

    Ok(())
}

#[derive(Accounts)]
#[instruction(amount: u64, nonce: u64)]
pub struct RequestWithdrawal<'info> {
    #[account(mut, constraint = signer.key() == vault.owner @ VaultError::NotVaultOwner)]
    pub signer: Signer<'info>,

    pub vault: Account<'info, Vault>,

    #[account(has_one = vault)]
    pub beneficiary_list: Account<'info, BeneficiaryList>,

    pub recipient_token_account: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = signer,
        space = PendingWithdrawal::SPACE,
        seeds = [b"request", vault.key().as_ref(), &nonce.to_le_bytes()],
        bump
    )]
    pub pending_withdrawal: Account<'info, PendingWithdrawal>,

    pub system_program: Program<'info, System>,
}
