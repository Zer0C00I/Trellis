use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::errors::VaultError;
use crate::state::*;

pub fn initialize_vault(
    ctx: Context<InitializeVault>,
    is_pool: bool,
    daily_limit: u64,
) -> Result<()> {
    if is_pool {
        require!(daily_limit > 0, VaultError::InvalidDailyLimit);
    }

    let vault = &mut ctx.accounts.vault;
    vault.owner = ctx.accounts.owner.key();
    vault.seed_owner = ctx.accounts.owner.key();
    vault.vault_token_account = ctx.accounts.vault_token_account.key();
    vault.daily_limit = if is_pool { daily_limit } else { u64::MAX };
    vault.spent_today = 0;
    vault.last_reset_day = 0;
    vault.is_pool = is_pool;
    vault.bump = ctx.bumps.vault;

    let list = &mut ctx.accounts.beneficiary_list;
    list.vault = vault.key();
    list.count = 0;

    Ok(())
}

#[derive(Accounts)]
#[instruction(is_pool: bool)]
pub struct InitializeVault<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub owner: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = Vault::SPACE,
        seeds = [b"vault".as_ref(), owner.key().as_ref(), if is_pool { b"pool".as_ref() } else { b"personal".as_ref() }],
        bump
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        init,
        payer = payer,
        space = BeneficiaryList::SPACE,
        seeds = [b"beneficiaries", vault.key().as_ref()],
        bump
    )]
    pub beneficiary_list: Account<'info, BeneficiaryList>,

    pub mint: Account<'info, Mint>,

    #[account(
        init,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = vault,
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
