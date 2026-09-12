mod common;

use anchor_lang::{InstructionData, ToAccountMetas};
use common::setup;
use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::{instruction::Instruction, system_program};
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::Transaction;

fn init_pool_vault(ctx: &mut common::TestCtx, owner: &Keypair, daily_limit: u64) -> Pubkey {
    let (vault, _) = Pubkey::find_program_address(&[b"vault", owner.pubkey().as_ref(), b"pool"], &family_vault::ID);
    let (beneficiary_list, _) = Pubkey::find_program_address(&[b"beneficiaries", vault.as_ref()], &family_vault::ID);
    let vault_token_account = spl_associated_token_account::get_associated_token_address(&vault, &ctx.mint);

    let init_accounts = family_vault::accounts::InitializeVault {
        payer: ctx.payer.pubkey(), owner: owner.pubkey(), vault, beneficiary_list,
        mint: ctx.mint, vault_token_account, token_program: spl_token::id(),
        associated_token_program: spl_associated_token_account::id(), system_program: system_program::id(),
    };
    let init_data = family_vault::instruction::InitializeVault { is_pool: true, daily_limit };
    let ix = Instruction { program_id: family_vault::ID, accounts: init_accounts.to_account_metas(None), data: init_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, owner], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx).unwrap();

    vault
}

fn init_approvers(ctx: &mut common::TestCtx, owner: &Keypair, vault: Pubkey) -> Pubkey {
    let (approvers, _) = Pubkey::find_program_address(&[b"approvers", vault.as_ref()], &family_vault::ID);
    let accounts = family_vault::accounts::InitializeApprovers { payer: owner.pubkey(), vault, approvers, system_program: system_program::id() };
    let data = family_vault::instruction::InitializeApprovers {};
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, owner], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "initialize_approvers failed: {:?}", result);
    approvers
}

#[test]
fn owner_can_initialize_and_add_a_trusted_approver() {
    let mut ctx = setup();
    let owner = Keypair::new();
    ctx.svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    let vault = init_pool_vault(&mut ctx, &owner, 50_000_000);
    let approvers = init_approvers(&mut ctx, &owner, vault);

    let approver_addr = Pubkey::new_unique();
    let accounts = family_vault::accounts::AddTrustedApprover { signer: owner.pubkey(), vault, approvers };
    let data = family_vault::instruction::AddTrustedApprover { address: approver_addr };
    let ix = Instruction { program_id: family_vault::ID, accounts: accounts.to_account_metas(None), data: data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &owner], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "add_trusted_approver failed: {:?}", result);

    let approvers_account = ctx.svm.get_account(&approvers).unwrap();
    let approvers_data: family_vault::state::TrustedApprovers =
        anchor_lang::AccountDeserialize::try_deserialize(&mut approvers_account.data.as_slice()).unwrap();
    assert_eq!(approvers_data.count, 1);
    assert_eq!(approvers_data.entries[0], approver_addr);
}

#[test]
fn non_owner_cannot_add_a_trusted_approver() {
    let mut ctx = setup();
    let owner = Keypair::new();
    ctx.svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    let vault = init_pool_vault(&mut ctx, &owner, 50_000_000);
    let approvers = init_approvers(&mut ctx, &owner, vault);

    let attacker = Keypair::new();
    ctx.svm.airdrop(&attacker.pubkey(), 1_000_000_000).unwrap();

    let accounts = family_vault::accounts::AddTrustedApprover { signer: attacker.pubkey(), vault, approvers };
    let data = family_vault::instruction::AddTrustedApprover { address: Pubkey::new_unique() };
    let ix = Instruction { program_id: family_vault::ID, accounts: accounts.to_account_metas(None), data: data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &attacker], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_err(), "expected non-owner add_trusted_approver to be rejected");
}

#[test]
fn rejects_duplicate_approver_address() {
    let mut ctx = setup();
    let owner = Keypair::new();
    ctx.svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    let vault = init_pool_vault(&mut ctx, &owner, 50_000_000);
    let approvers = init_approvers(&mut ctx, &owner, vault);

    let approver_addr = Pubkey::new_unique();
    for expect_ok in [true, false] {
        let accounts = family_vault::accounts::AddTrustedApprover { signer: owner.pubkey(), vault, approvers };
        let data = family_vault::instruction::AddTrustedApprover { address: approver_addr };
        let ix = Instruction { program_id: family_vault::ID, accounts: accounts.to_account_metas(None), data: data.data() };
        let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &owner], ctx.svm.latest_blockhash());
        let result = ctx.svm.send_transaction(tx);
        assert_eq!(result.is_ok(), expect_ok, "unexpected result for expect_ok={expect_ok}: {:?}", result);
    }
}
