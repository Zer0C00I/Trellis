use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::errors::VaultError;
use crate::state::{BeneficiaryList, Vault};

pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    require!(amount > 0, VaultError::ZeroAmount);

    let to_key = ctx.accounts.recipient_token_account.owner;
    let list = &ctx.accounts.beneficiary_list;
    let clock = Clock::get()?;

    let mut found = false;
    for i in 0..(list.count as usize) {
        let b = &list.entries[i];
        if b.address == to_key {
            require!(
                clock.unix_timestamp >= b.active_after,
                VaultError::BeneficiaryCooldownActive
            );
            found = true;
            break;
        }
    }
    require!(found, VaultError::UnknownBeneficiary);

    let vault = &mut ctx.accounts.vault;
    let today = clock.unix_timestamp / 86_400;
    if vault.is_pool {
        if vault.last_reset_day != today {
            vault.spent_today = 0;
            vault.last_reset_day = today;
        }
        let new_spent = vault
            .spent_today
            .checked_add(amount)
            .ok_or(VaultError::AmountOverflow)?;
        require!(
            new_spent <= vault.daily_limit,
            VaultError::DailyLimitExceeded
        );
        vault.spent_today = new_spent;
    }

    let owner_key = vault.seed_owner;
    let seed_kind: &[u8] = if vault.is_pool { b"pool" } else { b"personal" };
    let bump = vault.bump;
    let vault_seeds: &[&[u8]] = &[b"vault", owner_key.as_ref(), seed_kind, &[bump]];
    let signer_seeds: &[&[&[u8]]] = &[vault_seeds];

    let cpi_accounts = Transfer {
        from: ctx.accounts.vault_token_account.to_account_info(),
        to: ctx.accounts.recipient_token_account.to_account_info(),
        authority: vault.to_account_info(),
    };
    let cpi_ctx =
        CpiContext::new_with_signer(ctx.accounts.token_program.key(), cpi_accounts, signer_seeds);
    token::transfer(cpi_ctx, amount)
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(constraint = signer.key() == vault.owner @ VaultError::NotVaultOwner)]
    pub signer: Signer<'info>,

    #[account(mut, address = vault.vault_token_account)]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub recipient_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub vault: Account<'info, Vault>,

    #[account(has_one = vault)]
    pub beneficiary_list: Account<'info, BeneficiaryList>,

    pub token_program: Program<'info, Token>,
}
