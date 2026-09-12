mod common;

use anchor_lang::{InstructionData, ToAccountMetas};
use common::setup;
use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::{instruction::Instruction, system_program};
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::Transaction;

struct RotationFixture {
    vault: Pubkey,
    approvers: Pubkey,
}

fn setup_personal_vault_with_approver(ctx: &mut common::TestCtx, owner: &Keypair, approver: &Keypair) -> RotationFixture {
    let (vault, _) = Pubkey::find_program_address(&[b"vault", owner.pubkey().as_ref(), b"personal"], &family_vault::ID);
    let (beneficiary_list, _) = Pubkey::find_program_address(&[b"beneficiaries", vault.as_ref()], &family_vault::ID);
    let vault_token_account = spl_associated_token_account::get_associated_token_address(&vault, &ctx.mint);

    let init_accounts = family_vault::accounts::InitializeVault {
        payer: ctx.payer.pubkey(), owner: owner.pubkey(), vault, beneficiary_list,
        mint: ctx.mint, vault_token_account, token_program: spl_token::id(),
        associated_token_program: spl_associated_token_account::id(), system_program: system_program::id(),
    };
    let init_data = family_vault::instruction::InitializeVault { is_pool: false, daily_limit: 0 };
    let ix = Instruction { program_id: family_vault::ID, accounts: init_accounts.to_account_metas(None), data: init_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, owner], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx).unwrap();

    let (approvers, _) = Pubkey::find_program_address(&[b"approvers", vault.as_ref()], &family_vault::ID);
    let init_app_accounts = family_vault::accounts::InitializeApprovers { payer: owner.pubkey(), vault, approvers, system_program: system_program::id() };
    let ix = Instruction { program_id: family_vault::ID, accounts: init_app_accounts.to_account_metas(None), data: family_vault::instruction::InitializeApprovers {}.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, owner], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx).unwrap();

    let add_app_accounts = family_vault::accounts::AddTrustedApprover { signer: owner.pubkey(), vault, approvers };
    let add_app_data = family_vault::instruction::AddTrustedApprover { address: approver.pubkey() };
    let ix = Instruction { program_id: family_vault::ID, accounts: add_app_accounts.to_account_metas(None), data: add_app_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, owner], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx).unwrap();

    RotationFixture { vault, approvers }
}

fn vault_owner(ctx: &common::TestCtx, vault: &Pubkey) -> Pubkey {
    let account = ctx.svm.get_account(vault).unwrap();
    let data: family_vault::state::Vault =
        anchor_lang::AccountDeserialize::try_deserialize(&mut account.data.as_slice()).unwrap();
    data.owner
}

#[test]
fn approver_can_request_and_execute_rotation_after_timelock() {
    let mut ctx = setup();
    let owner = Keypair::new();
    let approver = Keypair::new();
    let new_owner = Pubkey::new_unique();
    ctx.svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    ctx.svm.airdrop(&approver.pubkey(), 1_000_000_000).unwrap();
    let fixture = setup_personal_vault_with_approver(&mut ctx, &owner, &approver);

    let nonce: u64 = 1;
    let (rotation, _) = Pubkey::find_program_address(
        &[b"rotation", fixture.vault.as_ref(), &nonce.to_le_bytes()],
        &family_vault::ID,
    );
    let request_accounts = family_vault::accounts::RequestOwnerRotation {
        signer: approver.pubkey(), vault: fixture.vault, approvers: fixture.approvers,
        rotation, system_program: system_program::id(),
    };
    let request_data = family_vault::instruction::RequestOwnerRotation { new_owner, nonce };
    let ix = Instruction { program_id: family_vault::ID, accounts: request_accounts.to_account_metas(None), data: request_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &approver], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "request_owner_rotation failed: {:?}", result);
    assert_eq!(vault_owner(&ctx, &fixture.vault), owner.pubkey(), "owner must not change on request alone");

    // Attempting to execute before the timelock elapses must fail.
    let execute_accounts = family_vault::accounts::ExecuteOwnerRotation { signer: ctx.payer.pubkey(), vault: fixture.vault, rotation };
    let ix = Instruction { program_id: family_vault::ID, accounts: execute_accounts.to_account_metas(None), data: family_vault::instruction::ExecuteOwnerRotation {}.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_err(), "expected execute_owner_rotation before the timelock to be rejected");

    // Warp past the 72h timelock. Also expire the blockhash so the retried
    // transaction (identical accounts/data/signer to the rejected attempt
    // above) gets a fresh signature instead of being deduped as
    // AlreadyProcessed by LiteSVM.
    let mut clock = ctx.svm.get_sysvar::<anchor_lang::solana_program::clock::Clock>();
    clock.unix_timestamp += 259_201;
    ctx.svm.set_sysvar(&clock);
    ctx.svm.expire_blockhash();

    let execute_accounts = family_vault::accounts::ExecuteOwnerRotation { signer: ctx.payer.pubkey(), vault: fixture.vault, rotation };
    let ix = Instruction { program_id: family_vault::ID, accounts: execute_accounts.to_account_metas(None), data: family_vault::instruction::ExecuteOwnerRotation {}.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "execute_owner_rotation after the timelock failed: {:?}", result);
    assert_eq!(vault_owner(&ctx, &fixture.vault), new_owner);
}

#[test]
fn owner_can_cancel_a_pending_rotation_before_it_executes() {
    let mut ctx = setup();
    let owner = Keypair::new();
    let approver = Keypair::new();
    let new_owner = Pubkey::new_unique();
    ctx.svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    ctx.svm.airdrop(&approver.pubkey(), 1_000_000_000).unwrap();
    let fixture = setup_personal_vault_with_approver(&mut ctx, &owner, &approver);

    let nonce: u64 = 1;
    let (rotation, _) = Pubkey::find_program_address(
        &[b"rotation", fixture.vault.as_ref(), &nonce.to_le_bytes()],
        &family_vault::ID,
    );
    let request_accounts = family_vault::accounts::RequestOwnerRotation {
        signer: approver.pubkey(), vault: fixture.vault, approvers: fixture.approvers,
        rotation, system_program: system_program::id(),
    };
    let request_data = family_vault::instruction::RequestOwnerRotation { new_owner, nonce };
    let ix = Instruction { program_id: family_vault::ID, accounts: request_accounts.to_account_metas(None), data: request_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &approver], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx).unwrap();

    // The legitimate owner cancels — this is the "false alarm / attack" recovery path.
    let cancel_accounts = family_vault::accounts::CancelOwnerRotation { signer: owner.pubkey(), vault: fixture.vault, approvers: fixture.approvers, rotation };
    let ix = Instruction { program_id: family_vault::ID, accounts: cancel_accounts.to_account_metas(None), data: family_vault::instruction::CancelOwnerRotation {}.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &owner], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "cancel_owner_rotation by owner failed: {:?}", result);

    // Warp past the timelock — execution must now fail because it was cancelled.
    let mut clock = ctx.svm.get_sysvar::<anchor_lang::solana_program::clock::Clock>();
    clock.unix_timestamp += 259_201;
    ctx.svm.set_sysvar(&clock);

    let execute_accounts = family_vault::accounts::ExecuteOwnerRotation { signer: ctx.payer.pubkey(), vault: fixture.vault, rotation };
    let ix = Instruction { program_id: family_vault::ID, accounts: execute_accounts.to_account_metas(None), data: family_vault::instruction::ExecuteOwnerRotation {}.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_err(), "expected execute_owner_rotation on a cancelled request to be rejected");
    assert_eq!(vault_owner(&ctx, &fixture.vault), owner.pubkey(), "owner must remain unchanged after cancellation");
}

#[test]
fn non_approver_cannot_request_a_rotation() {
    let mut ctx = setup();
    let owner = Keypair::new();
    let approver = Keypair::new();
    let stranger = Keypair::new();
    ctx.svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    ctx.svm.airdrop(&approver.pubkey(), 1_000_000_000).unwrap();
    ctx.svm.airdrop(&stranger.pubkey(), 1_000_000_000).unwrap();
    let fixture = setup_personal_vault_with_approver(&mut ctx, &owner, &approver);

    let nonce: u64 = 1;
    let (rotation, _) = Pubkey::find_program_address(
        &[b"rotation", fixture.vault.as_ref(), &nonce.to_le_bytes()],
        &family_vault::ID,
    );
    let request_accounts = family_vault::accounts::RequestOwnerRotation {
        signer: stranger.pubkey(), vault: fixture.vault, approvers: fixture.approvers,
        rotation, system_program: system_program::id(),
    };
    let request_data = family_vault::instruction::RequestOwnerRotation { new_owner: Pubkey::new_unique(), nonce };
    let ix = Instruction { program_id: family_vault::ID, accounts: request_accounts.to_account_metas(None), data: request_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &stranger], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_err(), "expected non-approver request_owner_rotation to be rejected");
}
