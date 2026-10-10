mod common;

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::{instruction::Instruction, system_program};
use anchor_lang::{InstructionData, ToAccountMetas};
use common::setup;
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::Transaction;

const DAILY_LIMIT: u64 = 50_000_000;

struct PoolVaultFixture {
    vault: Pubkey,
    vault_token_account: Pubkey,
    beneficiary_list: Pubkey,
    approvers: Pubkey,
}

fn setup_pool_vault_with_approver(
    ctx: &mut common::TestCtx,
    owner: &Keypair,
    approver: &Keypair,
    beneficiary: Pubkey,
) -> PoolVaultFixture {
    let (vault, _) = Pubkey::find_program_address(
        &[b"vault", owner.pubkey().as_ref(), b"pool"],
        &family_vault::ID,
    );
    let (beneficiary_list, _) =
        Pubkey::find_program_address(&[b"beneficiaries", vault.as_ref()], &family_vault::ID);
    let vault_token_account =
        spl_associated_token_account::get_associated_token_address(&vault, &ctx.mint);

    let init_accounts = family_vault::accounts::InitializeVault {
        payer: ctx.payer.pubkey(),
        owner: owner.pubkey(),
        vault,
        beneficiary_list,
        mint: ctx.mint,
        vault_token_account,
        token_program: spl_token::id(),
        associated_token_program: spl_associated_token_account::id(),
        system_program: system_program::id(),
    };
    let init_data = family_vault::instruction::InitializeVault {
        is_pool: true,
        daily_limit: DAILY_LIMIT,
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: init_accounts.to_account_metas(None),
        data: init_data.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, owner],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();

    let mint_ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        &ctx.mint,
        &vault_token_account,
        &ctx.mint_authority.pubkey(),
        &[],
        200_000_000,
    )
    .unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[mint_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &ctx.mint_authority],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();

    let add_ben_accounts = family_vault::accounts::AddBeneficiary {
        signer: owner.pubkey(),
        vault,
        beneficiary_list,
    };
    let add_ben_data = family_vault::instruction::AddBeneficiary {
        name: "Beneficiary".to_string(),
        address: beneficiary,
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: add_ben_accounts.to_account_metas(None),
        data: add_ben_data.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, owner],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();
    let mut clock = ctx
        .svm
        .get_sysvar::<anchor_lang::solana_program::clock::Clock>();
    clock.unix_timestamp += 172_801;
    ctx.svm.set_sysvar(&clock);

    let (approvers, _) =
        Pubkey::find_program_address(&[b"approvers", vault.as_ref()], &family_vault::ID);
    let init_app_accounts = family_vault::accounts::InitializeApprovers {
        payer: owner.pubkey(),
        vault,
        approvers,
        system_program: system_program::id(),
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: init_app_accounts.to_account_metas(None),
        data: family_vault::instruction::InitializeApprovers {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, owner],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();

    let add_app_accounts = family_vault::accounts::AddTrustedApprover {
        signer: owner.pubkey(),
        vault,
        approvers,
        existing_approver: None,
    };
    let add_app_data = family_vault::instruction::AddTrustedApprover {
        address: approver.pubkey(),
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: add_app_accounts.to_account_metas(None),
        data: add_app_data.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, owner],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();

    PoolVaultFixture {
        vault,
        vault_token_account,
        beneficiary_list,
        approvers,
    }
}

fn create_ata(ctx: &mut common::TestCtx, owner: &Pubkey) -> Pubkey {
    let ix = spl_associated_token_account::instruction::create_associated_token_account(
        &ctx.payer.pubkey(),
        owner,
        &ctx.mint,
        &spl_token::id(),
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();
    spl_associated_token_account::get_associated_token_address(owner, &ctx.mint)
}

fn token_balance(ctx: &common::TestCtx, ata: &Pubkey) -> u64 {
    let account = ctx.svm.get_account(ata).unwrap();
    let data: spl_token::state::Account =
        anchor_lang::solana_program::program_pack::Pack::unpack(&account.data).unwrap();
    data.amount
}

#[test]
fn approver_can_approve_and_execute_an_over_limit_withdrawal() {
    let mut ctx = setup();
    let owner = Keypair::new();
    let approver = Keypair::new();
    let beneficiary = Keypair::new();
    ctx.svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    ctx.svm.airdrop(&approver.pubkey(), 1_000_000_000).unwrap();
    ctx.svm
        .airdrop(&beneficiary.pubkey(), 1_000_000_000)
        .unwrap();

    let fixture = setup_pool_vault_with_approver(&mut ctx, &owner, &approver, beneficiary.pubkey());
    let beneficiary_ata = create_ata(&mut ctx, &beneficiary.pubkey());

    // Request 80M, which exceeds the 50M daily limit.
    let nonce: u64 = 1;
    let (pending_withdrawal, _) = Pubkey::find_program_address(
        &[b"request", fixture.vault.as_ref(), &nonce.to_le_bytes()],
        &family_vault::ID,
    );
    let request_accounts = family_vault::accounts::RequestWithdrawal {
        signer: owner.pubkey(),
        vault: fixture.vault,
        beneficiary_list: fixture.beneficiary_list,
        recipient_token_account: beneficiary_ata,
        pending_withdrawal,
        system_program: system_program::id(),
    };
    let request_data = family_vault::instruction::RequestWithdrawal {
        amount: 80_000_000,
        nonce,
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: request_accounts.to_account_metas(None),
        data: request_data.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &owner],
        ctx.svm.latest_blockhash(),
    );
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "request_withdrawal failed: {:?}", result);
    assert_eq!(
        token_balance(&ctx, &beneficiary_ata),
        0,
        "requesting must not move funds yet"
    );

    let approve_accounts = family_vault::accounts::ApproveWithdrawal {
        signer: approver.pubkey(),
        pending_withdrawal,
        approvers: fixture.approvers,
        vault_token_account: fixture.vault_token_account,
        recipient_token_account: beneficiary_ata,
        vault: fixture.vault,
        token_program: spl_token::id(),
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: approve_accounts.to_account_metas(None),
        data: family_vault::instruction::ApproveWithdrawal {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &approver],
        ctx.svm.latest_blockhash(),
    );
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "approve_withdrawal failed: {:?}", result);
    assert_eq!(token_balance(&ctx, &beneficiary_ata), 80_000_000);
}

#[test]
fn non_approver_cannot_approve_a_withdrawal() {
    let mut ctx = setup();
    let owner = Keypair::new();
    let approver = Keypair::new();
    let beneficiary = Keypair::new();
    let stranger = Keypair::new();
    ctx.svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    ctx.svm.airdrop(&approver.pubkey(), 1_000_000_000).unwrap();
    ctx.svm
        .airdrop(&beneficiary.pubkey(), 1_000_000_000)
        .unwrap();
    ctx.svm.airdrop(&stranger.pubkey(), 1_000_000_000).unwrap();

    let fixture = setup_pool_vault_with_approver(&mut ctx, &owner, &approver, beneficiary.pubkey());
    let beneficiary_ata = create_ata(&mut ctx, &beneficiary.pubkey());

    let nonce: u64 = 1;
    let (pending_withdrawal, _) = Pubkey::find_program_address(
        &[b"request", fixture.vault.as_ref(), &nonce.to_le_bytes()],
        &family_vault::ID,
    );
    let request_accounts = family_vault::accounts::RequestWithdrawal {
        signer: owner.pubkey(),
        vault: fixture.vault,
        beneficiary_list: fixture.beneficiary_list,
        recipient_token_account: beneficiary_ata,
        pending_withdrawal,
        system_program: system_program::id(),
    };
    let request_data = family_vault::instruction::RequestWithdrawal {
        amount: 80_000_000,
        nonce,
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: request_accounts.to_account_metas(None),
        data: request_data.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &owner],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();

    let approve_accounts = family_vault::accounts::ApproveWithdrawal {
        signer: stranger.pubkey(),
        pending_withdrawal,
        approvers: fixture.approvers,
        vault_token_account: fixture.vault_token_account,
        recipient_token_account: beneficiary_ata,
        vault: fixture.vault,
        token_program: spl_token::id(),
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: approve_accounts.to_account_metas(None),
        data: family_vault::instruction::ApproveWithdrawal {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &stranger],
        ctx.svm.latest_blockhash(),
    );
    let result = ctx.svm.send_transaction(tx);
    assert!(
        result.is_err(),
        "expected non-approver approve_withdrawal to be rejected"
    );
    assert_eq!(token_balance(&ctx, &beneficiary_ata), 0);
}

#[test]
fn cannot_approve_the_same_request_twice() {
    let mut ctx = setup();
    let owner = Keypair::new();
    let approver = Keypair::new();
    let beneficiary = Keypair::new();
    ctx.svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    ctx.svm.airdrop(&approver.pubkey(), 1_000_000_000).unwrap();
    ctx.svm
        .airdrop(&beneficiary.pubkey(), 1_000_000_000)
        .unwrap();

    let fixture = setup_pool_vault_with_approver(&mut ctx, &owner, &approver, beneficiary.pubkey());
    let beneficiary_ata = create_ata(&mut ctx, &beneficiary.pubkey());

    let nonce: u64 = 1;
    let (pending_withdrawal, _) = Pubkey::find_program_address(
        &[b"request", fixture.vault.as_ref(), &nonce.to_le_bytes()],
        &family_vault::ID,
    );
    let request_accounts = family_vault::accounts::RequestWithdrawal {
        signer: owner.pubkey(),
        vault: fixture.vault,
        beneficiary_list: fixture.beneficiary_list,
        recipient_token_account: beneficiary_ata,
        pending_withdrawal,
        system_program: system_program::id(),
    };
    let request_data = family_vault::instruction::RequestWithdrawal {
        amount: 60_000_000,
        nonce,
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: request_accounts.to_account_metas(None),
        data: request_data.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &owner],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();

    let approve_accounts = family_vault::accounts::ApproveWithdrawal {
        signer: approver.pubkey(),
        pending_withdrawal,
        approvers: fixture.approvers,
        vault_token_account: fixture.vault_token_account,
        recipient_token_account: beneficiary_ata,
        vault: fixture.vault,
        token_program: spl_token::id(),
    };
    let ix1 = Instruction {
        program_id: family_vault::ID,
        accounts: approve_accounts.to_account_metas(None),
        data: family_vault::instruction::ApproveWithdrawal {}.data(),
    };
    let tx1 = Transaction::new_signed_with_payer(
        &[ix1],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &approver],
        ctx.svm.latest_blockhash(),
    );
    let result1 = ctx.svm.send_transaction(tx1);
    assert!(
        result1.is_ok(),
        "first approve_withdrawal failed: {:?}",
        result1
    );

    let ix2 = Instruction {
        program_id: family_vault::ID,
        accounts: approve_accounts.to_account_metas(None),
        data: family_vault::instruction::ApproveWithdrawal {}.data(),
    };
    let tx2 = Transaction::new_signed_with_payer(
        &[ix2],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &approver],
        ctx.svm.latest_blockhash(),
    );
    let result2 = ctx.svm.send_transaction(tx2);
    assert!(
        result2.is_err(),
        "expected second approve_withdrawal on the same request to be rejected"
    );
    assert_eq!(
        token_balance(&ctx, &beneficiary_ata),
        60_000_000,
        "balance must reflect exactly one execution"
    );
}

#[test]
fn approver_cannot_redirect_recipient_to_a_different_token_account() {
    let mut ctx = setup();
    let owner = Keypair::new();
    let approver = Keypair::new();
    let beneficiary = Keypair::new();
    let attacker = Keypair::new();
    ctx.svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    ctx.svm.airdrop(&approver.pubkey(), 1_000_000_000).unwrap();
    ctx.svm
        .airdrop(&beneficiary.pubkey(), 1_000_000_000)
        .unwrap();
    ctx.svm.airdrop(&attacker.pubkey(), 1_000_000_000).unwrap();

    let fixture = setup_pool_vault_with_approver(&mut ctx, &owner, &approver, beneficiary.pubkey());
    let beneficiary_ata = create_ata(&mut ctx, &beneficiary.pubkey());
    // An account the attacker (a malicious or compromised approver) controls,
    // NOT the recipient named in the original request.
    let attacker_ata = create_ata(&mut ctx, &attacker.pubkey());

    let nonce: u64 = 1;
    let (pending_withdrawal, _) = Pubkey::find_program_address(
        &[b"request", fixture.vault.as_ref(), &nonce.to_le_bytes()],
        &family_vault::ID,
    );
    let request_accounts = family_vault::accounts::RequestWithdrawal {
        signer: owner.pubkey(),
        vault: fixture.vault,
        beneficiary_list: fixture.beneficiary_list,
        recipient_token_account: beneficiary_ata,
        pending_withdrawal,
        system_program: system_program::id(),
    };
    let request_data = family_vault::instruction::RequestWithdrawal {
        amount: 80_000_000,
        nonce,
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: request_accounts.to_account_metas(None),
        data: request_data.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &owner],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();

    // Approver substitutes attacker_ata instead of the originally-requested
    // beneficiary_ata — this must be rejected by the
    // `recipient_token_account.owner == pending_withdrawal.to` constraint.
    let approve_accounts = family_vault::accounts::ApproveWithdrawal {
        signer: approver.pubkey(),
        pending_withdrawal,
        approvers: fixture.approvers,
        vault_token_account: fixture.vault_token_account,
        recipient_token_account: attacker_ata,
        vault: fixture.vault,
        token_program: spl_token::id(),
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: approve_accounts.to_account_metas(None),
        data: family_vault::instruction::ApproveWithdrawal {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &approver],
        ctx.svm.latest_blockhash(),
    );
    let result = ctx.svm.send_transaction(tx);
    assert!(
        result.is_err(),
        "expected approve_withdrawal with a substituted recipient_token_account to be rejected"
    );
    assert_eq!(
        token_balance(&ctx, &attacker_ata),
        0,
        "no funds should have moved to the attacker's account"
    );
    assert_eq!(
        token_balance(&ctx, &beneficiary_ata),
        0,
        "no funds should have moved at all"
    );
}

#[test]
fn cross_vault_substitution_of_pending_withdrawal_and_approvers_is_rejected() {
    let mut ctx = setup();
    let owner_a = Keypair::new();
    let approver_a = Keypair::new();
    let beneficiary_a = Keypair::new();
    let owner_b = Keypair::new();
    let approver_b = Keypair::new();
    let beneficiary_b = Keypair::new();
    ctx.svm.airdrop(&owner_a.pubkey(), 1_000_000_000).unwrap();
    ctx.svm
        .airdrop(&approver_a.pubkey(), 1_000_000_000)
        .unwrap();
    ctx.svm
        .airdrop(&beneficiary_a.pubkey(), 1_000_000_000)
        .unwrap();
    ctx.svm.airdrop(&owner_b.pubkey(), 1_000_000_000).unwrap();
    ctx.svm
        .airdrop(&approver_b.pubkey(), 1_000_000_000)
        .unwrap();
    ctx.svm
        .airdrop(&beneficiary_b.pubkey(), 1_000_000_000)
        .unwrap();

    // Two independent pool vaults, each with their own approvers, beneficiary
    // and pending_withdrawal.
    let fixture_a =
        setup_pool_vault_with_approver(&mut ctx, &owner_a, &approver_a, beneficiary_a.pubkey());
    let beneficiary_a_ata = create_ata(&mut ctx, &beneficiary_a.pubkey());
    let fixture_b =
        setup_pool_vault_with_approver(&mut ctx, &owner_b, &approver_b, beneficiary_b.pubkey());
    let beneficiary_b_ata = create_ata(&mut ctx, &beneficiary_b.pubkey());

    let nonce: u64 = 1;
    let (pending_withdrawal_a, _) = Pubkey::find_program_address(
        &[b"request", fixture_a.vault.as_ref(), &nonce.to_le_bytes()],
        &family_vault::ID,
    );
    let request_accounts_a = family_vault::accounts::RequestWithdrawal {
        signer: owner_a.pubkey(),
        vault: fixture_a.vault,
        beneficiary_list: fixture_a.beneficiary_list,
        recipient_token_account: beneficiary_a_ata,
        pending_withdrawal: pending_withdrawal_a,
        system_program: system_program::id(),
    };
    let request_data_a = family_vault::instruction::RequestWithdrawal {
        amount: 80_000_000,
        nonce,
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: request_accounts_a.to_account_metas(None),
        data: request_data_a.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &owner_a],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();

    let (pending_withdrawal_b, _) = Pubkey::find_program_address(
        &[b"request", fixture_b.vault.as_ref(), &nonce.to_le_bytes()],
        &family_vault::ID,
    );
    let request_accounts_b = family_vault::accounts::RequestWithdrawal {
        signer: owner_b.pubkey(),
        vault: fixture_b.vault,
        beneficiary_list: fixture_b.beneficiary_list,
        recipient_token_account: beneficiary_b_ata,
        pending_withdrawal: pending_withdrawal_b,
        system_program: system_program::id(),
    };
    let request_data_b = family_vault::instruction::RequestWithdrawal {
        amount: 80_000_000,
        nonce,
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: request_accounts_b.to_account_metas(None),
        data: request_data_b.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &owner_b],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();

    // Attempt to use vault A's `approvers` and `pending_withdrawal` against
    // vault B's `approve_withdrawal` call (vault: fixture_b.vault). The
    // `has_one = vault` constraints on both accounts must reject this.
    let approve_accounts = family_vault::accounts::ApproveWithdrawal {
        signer: approver_a.pubkey(),
        pending_withdrawal: pending_withdrawal_a,
        approvers: fixture_a.approvers,
        vault_token_account: fixture_b.vault_token_account,
        recipient_token_account: beneficiary_a_ata,
        vault: fixture_b.vault,
        token_program: spl_token::id(),
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: approve_accounts.to_account_metas(None),
        data: family_vault::instruction::ApproveWithdrawal {}.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &approver_a],
        ctx.svm.latest_blockhash(),
    );
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_err(), "expected cross-vault substitution of pending_withdrawal/approvers against a different vault to be rejected");
    assert_eq!(
        token_balance(&ctx, &beneficiary_a_ata),
        0,
        "no funds should have moved on a rejected cross-vault substitution"
    );

    // Also attempt using vault B's approvers/pending_withdrawal but vault A's
    // `vault` account — the has_one constraints must reject this direction too.
    let approve_accounts2 = family_vault::accounts::ApproveWithdrawal {
        signer: approver_b.pubkey(),
        pending_withdrawal: pending_withdrawal_b,
        approvers: fixture_b.approvers,
        vault_token_account: fixture_a.vault_token_account,
        recipient_token_account: beneficiary_b_ata,
        vault: fixture_a.vault,
        token_program: spl_token::id(),
    };
    let ix2 = Instruction {
        program_id: family_vault::ID,
        accounts: approve_accounts2.to_account_metas(None),
        data: family_vault::instruction::ApproveWithdrawal {}.data(),
    };
    let tx2 = Transaction::new_signed_with_payer(
        &[ix2],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &approver_b],
        ctx.svm.latest_blockhash(),
    );
    let result2 = ctx.svm.send_transaction(tx2);
    assert!(
        result2.is_err(),
        "expected reverse-direction cross-vault substitution to be rejected"
    );
    assert_eq!(
        token_balance(&ctx, &beneficiary_b_ata),
        0,
        "no funds should have moved on a rejected cross-vault substitution"
    );
}

#[test]
fn request_withdrawal_rejects_amount_within_daily_limit() {
    let mut ctx = setup();
    let owner = Keypair::new();
    let approver = Keypair::new();
    let beneficiary = Keypair::new();
    ctx.svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    ctx.svm.airdrop(&approver.pubkey(), 1_000_000_000).unwrap();
    ctx.svm
        .airdrop(&beneficiary.pubkey(), 1_000_000_000)
        .unwrap();

    let fixture = setup_pool_vault_with_approver(&mut ctx, &owner, &approver, beneficiary.pubkey());
    let beneficiary_ata = create_ata(&mut ctx, &beneficiary.pubkey());

    // 10M does not exceed the 50M daily limit — request_withdrawal must not
    // be usable as an unlimited-amount bypass channel; callers within the
    // limit should use ordinary `withdraw` instead.
    let nonce: u64 = 1;
    let (pending_withdrawal, _) = Pubkey::find_program_address(
        &[b"request", fixture.vault.as_ref(), &nonce.to_le_bytes()],
        &family_vault::ID,
    );
    let request_accounts = family_vault::accounts::RequestWithdrawal {
        signer: owner.pubkey(),
        vault: fixture.vault,
        beneficiary_list: fixture.beneficiary_list,
        recipient_token_account: beneficiary_ata,
        pending_withdrawal,
        system_program: system_program::id(),
    };
    let request_data = family_vault::instruction::RequestWithdrawal {
        amount: 10_000_000,
        nonce,
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: request_accounts.to_account_metas(None),
        data: request_data.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &owner],
        ctx.svm.latest_blockhash(),
    );
    let result = ctx.svm.send_transaction(tx);
    assert!(
        result.is_err(),
        "expected request_withdrawal for an amount within the daily limit to be rejected"
    );
}

#[test]
fn request_withdrawal_rejects_personal_vault() {
    let mut ctx = setup();
    let owner = Keypair::new();
    let beneficiary = Keypair::new();
    ctx.svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    ctx.svm
        .airdrop(&beneficiary.pubkey(), 1_000_000_000)
        .unwrap();

    // Initialize a personal (non-pool) vault directly, bypassing the pool fixture.
    let (vault, _) = Pubkey::find_program_address(
        &[b"vault", owner.pubkey().as_ref(), b"personal"],
        &family_vault::ID,
    );
    let (beneficiary_list, _) =
        Pubkey::find_program_address(&[b"beneficiaries", vault.as_ref()], &family_vault::ID);
    let vault_token_account =
        spl_associated_token_account::get_associated_token_address(&vault, &ctx.mint);

    let init_accounts = family_vault::accounts::InitializeVault {
        payer: ctx.payer.pubkey(),
        owner: owner.pubkey(),
        vault,
        beneficiary_list,
        mint: ctx.mint,
        vault_token_account,
        token_program: spl_token::id(),
        associated_token_program: spl_associated_token_account::id(),
        system_program: system_program::id(),
    };
    let init_data = family_vault::instruction::InitializeVault {
        is_pool: false,
        daily_limit: DAILY_LIMIT,
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: init_accounts.to_account_metas(None),
        data: init_data.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &owner],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();

    let add_ben_accounts = family_vault::accounts::AddBeneficiary {
        signer: owner.pubkey(),
        vault,
        beneficiary_list,
    };
    let add_ben_data = family_vault::instruction::AddBeneficiary {
        name: "Beneficiary".to_string(),
        address: beneficiary.pubkey(),
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: add_ben_accounts.to_account_metas(None),
        data: add_ben_data.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &owner],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();
    let mut clock = ctx
        .svm
        .get_sysvar::<anchor_lang::solana_program::clock::Clock>();
    clock.unix_timestamp += 172_801;
    ctx.svm.set_sysvar(&clock);

    let beneficiary_ata = create_ata(&mut ctx, &beneficiary.pubkey());

    let nonce: u64 = 1;
    let (pending_withdrawal, _) = Pubkey::find_program_address(
        &[b"request", vault.as_ref(), &nonce.to_le_bytes()],
        &family_vault::ID,
    );
    let request_accounts = family_vault::accounts::RequestWithdrawal {
        signer: owner.pubkey(),
        vault,
        beneficiary_list,
        recipient_token_account: beneficiary_ata,
        pending_withdrawal,
        system_program: system_program::id(),
    };
    let request_data = family_vault::instruction::RequestWithdrawal {
        amount: 10_000_000,
        nonce,
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: request_accounts.to_account_metas(None),
        data: request_data.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &owner],
        ctx.svm.latest_blockhash(),
    );
    let result = ctx.svm.send_transaction(tx);
    assert!(
        result.is_err(),
        "expected request_withdrawal on a personal vault to be rejected with NotPoolVault"
    );
}
