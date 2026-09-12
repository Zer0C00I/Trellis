use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::errors::VaultError;
use crate::state::{PendingWithdrawal, TrustedApprovers, Vault};

pub fn approve_withdrawal(ctx: Context<ApproveWithdrawal>) -> Result<()> {
    require!(!ctx.accounts.pending_withdrawal.executed, VaultError::RequestAlreadyExecuted);

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

    let vault = &ctx.accounts.vault;
    let owner_key = vault.owner;
    let seed_kind: &[u8] = if vault.is_pool { b"pool" } else { b"personal" };
    let bump = vault.bump;
    let vault_seeds: &[&[u8]] = &[b"vault", owner_key.as_ref(), seed_kind, &[bump]];
    let signer_seeds: &[&[&[u8]]] = &[vault_seeds];

    let amount = ctx.accounts.pending_withdrawal.amount;

    let cpi_accounts = Transfer {
        from: ctx.accounts.vault_token_account.to_account_info(),
        to: ctx.accounts.recipient_token_account.to_account_info(),
        authority: vault.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        cpi_accounts,
        signer_seeds,
    );
    token::transfer(cpi_ctx, amount)?;

    let request = &mut ctx.accounts.pending_withdrawal;
    request.approved_by = Some(signer_key);
    request.executed = true;

    Ok(())
}

#[derive(Accounts)]
pub struct ApproveWithdrawal<'info> {
    pub signer: Signer<'info>,

    #[account(mut, has_one = vault)]
    pub pending_withdrawal: Account<'info, PendingWithdrawal>,

    #[account(has_one = vault)]
    pub approvers: Account<'info, TrustedApprovers>,

    #[account(mut, address = vault.vault_token_account)]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(mut, constraint = recipient_token_account.owner == pending_withdrawal.to @ VaultError::UnknownBeneficiary)]
    pub recipient_token_account: Account<'info, TokenAccount>,

    pub vault: Account<'info, Vault>,

    pub token_program: Program<'info, Token>,
}
