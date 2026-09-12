mod common;

use anchor_lang::{InstructionData, ToAccountMetas};
use common::setup;
use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::{instruction::Instruction, system_program};
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::Transaction;

fn init_personal_vault(ctx: &mut common::TestCtx, elder: &Keypair) -> (Pubkey, Pubkey) {
    let (vault, _) = Pubkey::find_program_address(&[b"vault", elder.pubkey().as_ref(), b"personal"], &family_vault::ID);
    let (beneficiary_list, _) = Pubkey::find_program_address(&[b"beneficiaries", vault.as_ref()], &family_vault::ID);
    let vault_token_account = spl_associated_token_account::get_associated_token_address(&vault, &ctx.mint);
    let accounts = family_vault::accounts::InitializeVault {
        payer: ctx.payer.pubkey(), owner: elder.pubkey(), vault, beneficiary_list,
        mint: ctx.mint, vault_token_account, token_program: spl_token::id(),
        associated_token_program: spl_associated_token_account::id(), system_program: system_program::id(),
    };
    let data = family_vault::instruction::InitializeVault { is_pool: false, daily_limit: 0 };
    let ix = Instruction { program_id: family_vault::ID, accounts: accounts.to_account_metas(None), data: data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx).unwrap();
    (vault, beneficiary_list)
}

#[test]
fn owner_can_add_a_beneficiary_with_cooldown() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();
    let (vault, beneficiary_list) = init_personal_vault(&mut ctx, &elder);

    let grandson = Pubkey::new_unique();
    let accounts = family_vault::accounts::AddBeneficiary { signer: elder.pubkey(), vault, beneficiary_list };
    let data = family_vault::instruction::AddBeneficiary { name: "Grandson Sasha".to_string(), address: grandson };
    let ix = Instruction { program_id: family_vault::ID, accounts: accounts.to_account_metas(None), data: data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &elder], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "add_beneficiary failed: {:?}", result);

    let list_account = ctx.svm.get_account(&beneficiary_list).unwrap();
    let list_data: family_vault::state::BeneficiaryList =
        anchor_lang::AccountDeserialize::try_deserialize(&mut list_account.data.as_slice()).unwrap();
    assert_eq!(list_data.count, 1);
    assert_eq!(list_data.entries[0].address, grandson);
    assert!(list_data.entries[0].active_after > list_data.entries[0].added_at);
}

#[test]
fn non_owner_cannot_add_a_beneficiary() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();
    let (vault, beneficiary_list) = init_personal_vault(&mut ctx, &elder);

    let attacker = Keypair::new();
    ctx.svm.airdrop(&attacker.pubkey(), 1_000_000_000).unwrap();

    let grandson = Pubkey::new_unique();
    let accounts = family_vault::accounts::AddBeneficiary { signer: attacker.pubkey(), vault, beneficiary_list };
    let data = family_vault::instruction::AddBeneficiary { name: "Grandson Sasha".to_string(), address: grandson };
    let ix = Instruction { program_id: family_vault::ID, accounts: accounts.to_account_metas(None), data: data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &attacker], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_err(), "expected non-owner add_beneficiary to be rejected");

    let list_account = ctx.svm.get_account(&beneficiary_list).unwrap();
    let list_data: family_vault::state::BeneficiaryList =
        anchor_lang::AccountDeserialize::try_deserialize(&mut list_account.data.as_slice()).unwrap();
    assert_eq!(list_data.count, 0, "beneficiary list should be unchanged after rejected non-owner call");
}

#[test]
fn rejects_beneficiary_when_list_is_full() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();
    let (vault, beneficiary_list) = init_personal_vault(&mut ctx, &elder);

    // MAX_BENEFICIARIES is 10 — fill the list.
    for i in 0..10 {
        let addr = Pubkey::new_unique();
        let accounts = family_vault::accounts::AddBeneficiary { signer: elder.pubkey(), vault, beneficiary_list };
        let data = family_vault::instruction::AddBeneficiary { name: format!("Beneficiary {}", i), address: addr };
        let ix = Instruction { program_id: family_vault::ID, accounts: accounts.to_account_metas(None), data: data.data() };
        let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &elder], ctx.svm.latest_blockhash());
        let result = ctx.svm.send_transaction(tx);
        assert!(result.is_ok(), "expected beneficiary #{} to be added successfully: {:?}", i, result);
    }

    // 11th beneficiary should be rejected with BeneficiaryListFull.
    let overflow_addr = Pubkey::new_unique();
    let accounts = family_vault::accounts::AddBeneficiary { signer: elder.pubkey(), vault, beneficiary_list };
    let data = family_vault::instruction::AddBeneficiary { name: "Overflow".to_string(), address: overflow_addr };
    let ix = Instruction { program_id: family_vault::ID, accounts: accounts.to_account_metas(None), data: data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &elder], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_err(), "expected 11th beneficiary to be rejected when list is full");

    let list_account = ctx.svm.get_account(&beneficiary_list).unwrap();
    let list_data: family_vault::state::BeneficiaryList =
        anchor_lang::AccountDeserialize::try_deserialize(&mut list_account.data.as_slice()).unwrap();
    assert_eq!(list_data.count, 10, "beneficiary list should stay at capacity after rejected overflow call");
}

#[test]
fn rejects_duplicate_beneficiary_address() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();
    let (vault, beneficiary_list) = init_personal_vault(&mut ctx, &elder);

    let grandson = Pubkey::new_unique();
    let accounts = family_vault::accounts::AddBeneficiary { signer: elder.pubkey(), vault, beneficiary_list };
    let data = family_vault::instruction::AddBeneficiary { name: "Grandson Sasha".to_string(), address: grandson };
    let ix = Instruction { program_id: family_vault::ID, accounts: accounts.to_account_metas(None), data: data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &elder], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "expected first add_beneficiary to succeed: {:?}", result);

    // Same address, different name — should be rejected as a duplicate.
    let accounts = family_vault::accounts::AddBeneficiary { signer: elder.pubkey(), vault, beneficiary_list };
    let data = family_vault::instruction::AddBeneficiary { name: "Grandson Sasha Jr".to_string(), address: grandson };
    let ix = Instruction { program_id: family_vault::ID, accounts: accounts.to_account_metas(None), data: data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &elder], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_err(), "expected duplicate beneficiary address to be rejected");

    let list_account = ctx.svm.get_account(&beneficiary_list).unwrap();
    let list_data: family_vault::state::BeneficiaryList =
        anchor_lang::AccountDeserialize::try_deserialize(&mut list_account.data.as_slice()).unwrap();
    assert_eq!(list_data.count, 1, "beneficiary list should be unchanged after rejected duplicate call");
}
