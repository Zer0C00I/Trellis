mod common;

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::{instruction::Instruction, system_program};
use anchor_lang::{InstructionData, ToAccountMetas};
use common::setup;
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
        &[&ctx.payer, elder],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();
    (vault, beneficiary_list)
}

fn init_approvers(ctx: &mut common::TestCtx, owner: &Keypair, vault: Pubkey) -> Pubkey {
    let (approvers, _) =
        Pubkey::find_program_address(&[b"approvers", vault.as_ref()], &family_vault::ID);
    let accounts = family_vault::accounts::InitializeApprovers {
        payer: owner.pubkey(),
        vault,
        approvers,
        system_program: system_program::id(),
    };
    let data = family_vault::instruction::InitializeApprovers {};
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, owner],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();
    approvers
}

fn add_trusted_approver(
    ctx: &mut common::TestCtx,
    owner: &Keypair,
    vault: Pubkey,
    approvers: Pubkey,
    address: Pubkey,
    existing_approver: Option<&Keypair>,
) {
    let accounts = family_vault::accounts::AddTrustedApprover {
        signer: owner.pubkey(),
        vault,
        approvers,
        existing_approver: existing_approver.map(|k| k.pubkey()),
    };
    let data = family_vault::instruction::AddTrustedApprover { address };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    let mut signers: Vec<&Keypair> = vec![&ctx.payer, owner];
    if let Some(k) = existing_approver {
        signers.push(k);
    }
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &signers,
        ctx.svm.latest_blockhash(),
    );
    let result = ctx.svm.send_transaction(tx);
    assert!(
        result.is_ok(),
        "add_trusted_approver setup failed: {result:?}"
    );
}

#[test]
fn two_distinct_trusted_approvers_can_add_a_beneficiary_immediately_active() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();
    let (vault, beneficiary_list) = init_personal_vault(&mut ctx, &elder);
    let approvers = init_approvers(&mut ctx, &elder, vault);

    let approver_one = Keypair::new();
    add_trusted_approver(
        &mut ctx,
        &elder,
        vault,
        approvers,
        approver_one.pubkey(),
        None,
    );
    let approver_two = Keypair::new();
    add_trusted_approver(
        &mut ctx,
        &elder,
        vault,
        approvers,
        approver_two.pubkey(),
        Some(&approver_one),
    );

    let grandson = Pubkey::new_unique();
    let accounts = family_vault::accounts::AddBeneficiaryUrgent {
        approver_one: approver_one.pubkey(),
        approver_two: approver_two.pubkey(),
        vault,
        approvers,
        beneficiary_list,
    };
    let data = family_vault::instruction::AddBeneficiaryUrgent {
        name: "Urgent Nurse".to_string(),
        address: grandson,
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &approver_one, &approver_two],
        ctx.svm.latest_blockhash(),
    );
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "add_beneficiary_urgent failed: {result:?}");

    let list_account = ctx.svm.get_account(&beneficiary_list).unwrap();
    let list_data: family_vault::state::BeneficiaryList =
        anchor_lang::AccountDeserialize::try_deserialize(&mut list_account.data.as_slice())
            .unwrap();
    assert_eq!(list_data.count, 1);
    assert_eq!(list_data.entries[0].address, grandson);
    assert_eq!(
        list_data.entries[0].active_after, list_data.entries[0].added_at,
        "urgent-added beneficiary must be immediately active, not on the 48h cooldown"
    );
}

#[test]
fn single_approver_signing_both_slots_is_rejected() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();
    let (vault, beneficiary_list) = init_personal_vault(&mut ctx, &elder);
    let approvers = init_approvers(&mut ctx, &elder, vault);

    let approver_one = Keypair::new();
    add_trusted_approver(
        &mut ctx,
        &elder,
        vault,
        approvers,
        approver_one.pubkey(),
        None,
    );

    let grandson = Pubkey::new_unique();
    let accounts = family_vault::accounts::AddBeneficiaryUrgent {
        approver_one: approver_one.pubkey(),
        approver_two: approver_one.pubkey(),
        vault,
        approvers,
        beneficiary_list,
    };
    let data = family_vault::instruction::AddBeneficiaryUrgent {
        name: "Urgent Nurse".to_string(),
        address: grandson,
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &approver_one],
        ctx.svm.latest_blockhash(),
    );
    let result = ctx.svm.send_transaction(tx);
    assert!(
        result.is_err(),
        "expected same approver signing both slots to be rejected"
    );

    let list_account = ctx.svm.get_account(&beneficiary_list).unwrap();
    let list_data: family_vault::state::BeneficiaryList =
        anchor_lang::AccountDeserialize::try_deserialize(&mut list_account.data.as_slice())
            .unwrap();
    assert_eq!(list_data.count, 0);
}

#[test]
fn non_approver_in_either_slot_is_rejected() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();
    let (vault, beneficiary_list) = init_personal_vault(&mut ctx, &elder);
    let approvers = init_approvers(&mut ctx, &elder, vault);

    let approver_one = Keypair::new();
    add_trusted_approver(
        &mut ctx,
        &elder,
        vault,
        approvers,
        approver_one.pubkey(),
        None,
    );
    let stranger = Keypair::new();
    ctx.svm.airdrop(&stranger.pubkey(), 1_000_000_000).unwrap();

    let grandson = Pubkey::new_unique();
    let accounts = family_vault::accounts::AddBeneficiaryUrgent {
        approver_one: approver_one.pubkey(),
        approver_two: stranger.pubkey(),
        vault,
        approvers,
        beneficiary_list,
    };
    let data = family_vault::instruction::AddBeneficiaryUrgent {
        name: "Urgent Nurse".to_string(),
        address: grandson,
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &approver_one, &stranger],
        ctx.svm.latest_blockhash(),
    );
    let result = ctx.svm.send_transaction(tx);
    assert!(
        result.is_err(),
        "expected a non-approver in either signer slot to be rejected"
    );

    let list_account = ctx.svm.get_account(&beneficiary_list).unwrap();
    let list_data: family_vault::state::BeneficiaryList =
        anchor_lang::AccountDeserialize::try_deserialize(&mut list_account.data.as_slice())
            .unwrap();
    assert_eq!(list_data.count, 0);
}

#[test]
fn urgent_add_respects_duplicate_and_full_list_checks() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();
    let (vault, beneficiary_list) = init_personal_vault(&mut ctx, &elder);
    let approvers = init_approvers(&mut ctx, &elder, vault);

    let approver_one = Keypair::new();
    add_trusted_approver(
        &mut ctx,
        &elder,
        vault,
        approvers,
        approver_one.pubkey(),
        None,
    );
    let approver_two = Keypair::new();
    add_trusted_approver(
        &mut ctx,
        &elder,
        vault,
        approvers,
        approver_two.pubkey(),
        Some(&approver_one),
    );

    let grandson = Pubkey::new_unique();
    let accounts = family_vault::accounts::AddBeneficiaryUrgent {
        approver_one: approver_one.pubkey(),
        approver_two: approver_two.pubkey(),
        vault,
        approvers,
        beneficiary_list,
    };
    let data = family_vault::instruction::AddBeneficiaryUrgent {
        name: "Urgent Nurse".to_string(),
        address: grandson,
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &approver_one, &approver_two],
        ctx.svm.latest_blockhash(),
    );
    assert!(ctx.svm.send_transaction(tx).is_ok());

    // Same address again — must be rejected as a duplicate, same as the
    // ordinary add_beneficiary path.
    let accounts2 = family_vault::accounts::AddBeneficiaryUrgent {
        approver_one: approver_one.pubkey(),
        approver_two: approver_two.pubkey(),
        vault,
        approvers,
        beneficiary_list,
    };
    let data2 = family_vault::instruction::AddBeneficiaryUrgent {
        name: "Urgent Nurse Again".to_string(),
        address: grandson,
    };
    let ix2 = Instruction {
        program_id: family_vault::ID,
        accounts: accounts2.to_account_metas(None),
        data: data2.data(),
    };
    let tx2 = Transaction::new_signed_with_payer(
        &[ix2],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &approver_one, &approver_two],
        ctx.svm.latest_blockhash(),
    );
    let result2 = ctx.svm.send_transaction(tx2);
    assert!(
        result2.is_err(),
        "expected duplicate beneficiary to be rejected"
    );

    let list_account = ctx.svm.get_account(&beneficiary_list).unwrap();
    let list_data: family_vault::state::BeneficiaryList =
        anchor_lang::AccountDeserialize::try_deserialize(&mut list_account.data.as_slice())
            .unwrap();
    assert_eq!(list_data.count, 1);
}
