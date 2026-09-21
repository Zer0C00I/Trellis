mod common;

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::{instruction::Instruction, system_program};
use anchor_lang::{InstructionData, ToAccountMetas};
use common::setup;
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::Transaction;

#[test]
fn initializes_a_personal_vault_with_unlimited_daily_limit() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();

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

    let data = family_vault::instruction::InitializeVault {
        is_pool: false,
        daily_limit: 0,
    };

    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &elder],
        ctx.svm.latest_blockhash(),
    );
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "initialize_vault failed: {:?}", result);

    let vault_account = ctx.svm.get_account(&vault).unwrap();
    let vault_data: family_vault::state::Vault =
        anchor_lang::AccountDeserialize::try_deserialize(&mut vault_account.data.as_slice())
            .unwrap();

    assert_eq!(vault_data.owner, elder.pubkey());
    assert!(!vault_data.is_pool);
    assert_eq!(vault_data.daily_limit, u64::MAX);
    assert_eq!(vault_data.spent_today, 0);
}

#[test]
fn rejects_initialization_without_owner_signature() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();

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

    let data = family_vault::instruction::InitializeVault {
        is_pool: false,
        daily_limit: 0,
    };

    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };

    // Attempt to init a vault for elder's pubkey using only `payer` as a
    // signer — elder never co-signs, so this must be rejected now that
    // `owner` is a required Signer rather than an UncheckedAccount.
    // `Transaction::new_signed_with_payer` panics if not all required
    // signers are supplied, so build the transaction manually with only a
    // partial (payer-only) signature and let submission reject it instead.
    let mut tx = Transaction::new_with_payer(&[ix], Some(&ctx.payer.pubkey()));
    tx.partial_sign_unchecked(&[&ctx.payer], vec![0], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(
        result.is_err(),
        "expected initialize_vault without owner signature to be rejected"
    );
}
