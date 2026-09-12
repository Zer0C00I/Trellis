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
