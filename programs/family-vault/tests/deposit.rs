mod common;

use anchor_lang::{InstructionData, ToAccountMetas};
use common::setup;
use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::{instruction::Instruction, system_program};
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::Transaction;

fn init_personal_vault(ctx: &mut common::TestCtx, elder: &Keypair) -> (Pubkey, Pubkey) {
    let (vault, _) = Pubkey::find_program_address(
        &[b"vault", elder.pubkey().as_ref(), b"personal"],
        &family_vault::ID,
    );
    let (beneficiary_list, _) =
        Pubkey::find_program_address(&[b"beneficiaries", vault.as_ref()], &family_vault::ID);
    let vault_token_account =
        spl_associated_token_account::get_associated_token_address(&vault, &ctx.mint);

    let accounts = family_vault::accounts::InitializeVault {
        payer: ctx.payer.pubkey(),
        owner: elder.pubkey(),
        vault,
        beneficiary_list,
        mint: ctx.mint,
        vault_token_account,
        token_program: spl_token::id(),
        associated_token_program: spl_associated_token_account::id(),
        system_program: system_program::id(),
    };
    let data = family_vault::instruction::InitializeVault { is_pool: false, daily_limit: 0 };
    let ix = Instruction { program_id: family_vault::ID, accounts: accounts.to_account_metas(None), data: data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx).unwrap();

    (vault, vault_token_account)
}

#[test]
fn owner_can_deposit_into_own_personal_vault() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();
    let (vault, vault_token_account) = init_personal_vault(&mut ctx, &elder);

    // Create elder's own token account and mint 100 test tokens (6 decimals) into it.
    let elder_ata = spl_associated_token_account::get_associated_token_address(&elder.pubkey(), &ctx.mint);
    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &ctx.payer.pubkey(), &elder.pubkey(), &ctx.mint, &spl_token::id(),
    );
    let mint_to_ix = spl_token::instruction::mint_to(
        &spl_token::id(), &ctx.mint, &elder_ata, &ctx.mint_authority.pubkey(), &[], 100_000_000,
    ).unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_ata_ix, mint_to_ix], Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &ctx.mint_authority], ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();

    let deposit_accounts = family_vault::accounts::Deposit {
        depositor: elder.pubkey(),
        depositor_token_account: elder_ata,
        vault_token_account,
        vault,
        token_program: spl_token::id(),
    };
    let deposit_data = family_vault::instruction::Deposit { amount: 40_000_000 };
    let ix = Instruction { program_id: family_vault::ID, accounts: deposit_accounts.to_account_metas(None), data: deposit_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &elder], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "deposit failed: {:?}", result);

    let vault_ata_account = ctx.svm.get_account(&vault_token_account).unwrap();
    let vault_ata_data: spl_token::state::Account =
        anchor_lang::solana_program::program_pack::Pack::unpack(&vault_ata_account.data).unwrap();
    assert_eq!(vault_ata_data.amount, 40_000_000);
}

#[test]
fn non_owner_cannot_deposit_into_personal_vault() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();
    let (vault, vault_token_account) = init_personal_vault(&mut ctx, &elder);

    let attacker = Keypair::new();
    ctx.svm.airdrop(&attacker.pubkey(), 1_000_000_000).unwrap();

    // Create the attacker's own token account (no need to mint into it — the
    // owner-only check must reject the transaction before the CPI transfer runs).
    let attacker_ata = spl_associated_token_account::get_associated_token_address(&attacker.pubkey(), &ctx.mint);
    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &ctx.payer.pubkey(), &attacker.pubkey(), &ctx.mint, &spl_token::id(),
    );
    let tx = Transaction::new_signed_with_payer(
        &[create_ata_ix], Some(&ctx.payer.pubkey()),
        &[&ctx.payer], ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();

    let deposit_accounts = family_vault::accounts::Deposit {
        depositor: attacker.pubkey(),
        depositor_token_account: attacker_ata,
        vault_token_account,
        vault,
        token_program: spl_token::id(),
    };
    let deposit_data = family_vault::instruction::Deposit { amount: 40_000_000 };
    let ix = Instruction { program_id: family_vault::ID, accounts: deposit_accounts.to_account_metas(None), data: deposit_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &attacker], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_err(), "expected non-owner deposit into personal vault to be rejected");
}
