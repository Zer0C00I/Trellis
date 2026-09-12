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

    let add_app_accounts = family_vault::accounts::AddTrustedApprover { signer: owner.pubkey(), vault, approvers, existing_approver: None };
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
    let new_owner = Keypair::new();
    ctx.svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    ctx.svm.airdrop(&approver.pubkey(), 1_000_000_000).unwrap();
    ctx.svm.airdrop(&new_owner.pubkey(), 1_000_000_000).unwrap();
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
    let request_data = family_vault::instruction::RequestOwnerRotation { new_owner: new_owner.pubkey(), nonce };
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
    assert_eq!(vault_owner(&ctx, &fixture.vault), new_owner.pubkey());

    // Prove the rotation transferred *real* owner authority, not just the
    // stored pubkey field: the new owner must now be able to exercise owner
    // authority (add_trusted_approver), and the old owner must have lost it.
    let another_approver = Keypair::new();

    // Adding a second approver to a personal vault requires an existing
    // approver's co-signature (anti-takeover control), so `approver` co-signs.
    let new_owner_add_accounts = family_vault::accounts::AddTrustedApprover {
        signer: new_owner.pubkey(), vault: fixture.vault, approvers: fixture.approvers,
        existing_approver: Some(approver.pubkey()),
    };
    let new_owner_add_data = family_vault::instruction::AddTrustedApprover { address: another_approver.pubkey() };
    let ix = Instruction { program_id: family_vault::ID, accounts: new_owner_add_accounts.to_account_metas(None), data: new_owner_add_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &new_owner, &approver], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "new owner should be able to add_trusted_approver after rotation: {:?}", result);

    let old_owner_add_accounts = family_vault::accounts::AddTrustedApprover {
        signer: owner.pubkey(), vault: fixture.vault, approvers: fixture.approvers,
        existing_approver: None,
    };
    let old_owner_add_data = family_vault::instruction::AddTrustedApprover { address: Pubkey::new_unique() };
    let ix = Instruction { program_id: family_vault::ID, accounts: old_owner_add_accounts.to_account_metas(None), data: old_owner_add_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &owner], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_err(), "old owner must no longer be able to add_trusted_approver after rotation");
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

#[test]
fn approver_cannot_name_self_as_new_owner() {
    let mut ctx = setup();
    let owner = Keypair::new();
    let approver = Keypair::new();
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
    // The approver names itself as the new owner — a single unilateral
    // takeover step this must reject.
    let request_data = family_vault::instruction::RequestOwnerRotation { new_owner: approver.pubkey(), nonce };
    let ix = Instruction { program_id: family_vault::ID, accounts: request_accounts.to_account_metas(None), data: request_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &approver], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_err(), "expected request_owner_rotation naming the requester itself as new_owner to be rejected");
}

/// Regression test for the fund-freezing bug: the vault PDA's signer seeds
/// must be reconstructed from the ORIGINAL (`seed_owner`) owner, never the
/// current (mutable) `owner`. Before the fix, any withdrawal after a
/// completed rotation would fail with PrivilegeEscalation forever because
/// the reconstructed seeds no longer matched the PDA's real address.
#[test]
fn new_owner_can_withdraw_real_funds_after_rotation() {
    let mut ctx = setup();
    let owner = Keypair::new();
    let approver = Keypair::new();
    let new_owner = Keypair::new();
    let beneficiary = Keypair::new();
    ctx.svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    ctx.svm.airdrop(&approver.pubkey(), 1_000_000_000).unwrap();
    ctx.svm.airdrop(&new_owner.pubkey(), 1_000_000_000).unwrap();
    ctx.svm.airdrop(&beneficiary.pubkey(), 1_000_000_000).unwrap();

    let fixture = setup_personal_vault_with_approver(&mut ctx, &owner, &approver);
    let vault_token_account = spl_associated_token_account::get_associated_token_address(&fixture.vault, &ctx.mint);
    let (beneficiary_list, _) = Pubkey::find_program_address(&[b"beneficiaries", fixture.vault.as_ref()], &family_vault::ID);

    // Fund the vault with real tokens.
    let mint_ix = spl_token::instruction::mint_to(
        &spl_token::id(), &ctx.mint, &vault_token_account, &ctx.mint_authority.pubkey(), &[], 100_000_000,
    ).unwrap();
    let tx = Transaction::new_signed_with_payer(&[mint_ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &ctx.mint_authority], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx).unwrap();

    // Add a beneficiary and clear its cooldown.
    let add_ben_accounts = family_vault::accounts::AddBeneficiary { signer: owner.pubkey(), vault: fixture.vault, beneficiary_list };
    let add_ben_data = family_vault::instruction::AddBeneficiary { name: "Beneficiary".to_string(), address: beneficiary.pubkey() };
    let ix = Instruction { program_id: family_vault::ID, accounts: add_ben_accounts.to_account_metas(None), data: add_ben_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &owner], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx).unwrap();

    let mut clock = ctx.svm.get_sysvar::<anchor_lang::solana_program::clock::Clock>();
    clock.unix_timestamp += 172_801;
    ctx.svm.set_sysvar(&clock);

    // Request and execute owner rotation to `new_owner`.
    let nonce: u64 = 1;
    let (rotation, _) = Pubkey::find_program_address(
        &[b"rotation", fixture.vault.as_ref(), &nonce.to_le_bytes()],
        &family_vault::ID,
    );
    let request_accounts = family_vault::accounts::RequestOwnerRotation {
        signer: approver.pubkey(), vault: fixture.vault, approvers: fixture.approvers,
        rotation, system_program: system_program::id(),
    };
    let request_data = family_vault::instruction::RequestOwnerRotation { new_owner: new_owner.pubkey(), nonce };
    let ix = Instruction { program_id: family_vault::ID, accounts: request_accounts.to_account_metas(None), data: request_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &approver], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx).unwrap();

    let mut clock = ctx.svm.get_sysvar::<anchor_lang::solana_program::clock::Clock>();
    clock.unix_timestamp += 259_201;
    ctx.svm.set_sysvar(&clock);

    let execute_accounts = family_vault::accounts::ExecuteOwnerRotation { signer: ctx.payer.pubkey(), vault: fixture.vault, rotation };
    let ix = Instruction { program_id: family_vault::ID, accounts: execute_accounts.to_account_metas(None), data: family_vault::instruction::ExecuteOwnerRotation {}.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "execute_owner_rotation failed: {:?}", result);
    assert_eq!(vault_owner(&ctx, &fixture.vault), new_owner.pubkey());

    // The NEW owner must now be able to withdraw real tokens. Before the
    // `seed_owner` fix, this would fail with PrivilegeEscalation because the
    // signer seeds were rebuilt from the (now-changed) `vault.owner` instead
    // of the original PDA-derivation owner.
    let beneficiary_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &ctx.payer.pubkey(), &beneficiary.pubkey(), &ctx.mint, &spl_token::id(),
    );
    let tx0 = Transaction::new_signed_with_payer(&[beneficiary_ata_ix], Some(&ctx.payer.pubkey()), &[&ctx.payer], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx0).unwrap();
    let beneficiary_ata = spl_associated_token_account::get_associated_token_address(&beneficiary.pubkey(), &ctx.mint);

    let withdraw_accounts = family_vault::accounts::Withdraw {
        signer: new_owner.pubkey(), vault_token_account, recipient_token_account: beneficiary_ata,
        vault: fixture.vault, beneficiary_list, token_program: spl_token::id(),
    };
    let withdraw_data = family_vault::instruction::Withdraw { amount: 25_000_000 };
    let ix = Instruction { program_id: family_vault::ID, accounts: withdraw_accounts.to_account_metas(None), data: withdraw_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &new_owner], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "new owner's withdraw after rotation failed (fund-freezing regression): {:?}", result);

    let beneficiary_ata_account = ctx.svm.get_account(&beneficiary_ata).unwrap();
    let beneficiary_ata_data: spl_token::state::Account =
        anchor_lang::solana_program::program_pack::Pack::unpack(&beneficiary_ata_account.data).unwrap();
    assert_eq!(beneficiary_ata_data.amount, 25_000_000, "real tokens must have moved to the beneficiary");
}

#[test]
fn execute_owner_rotation_cannot_be_replayed() {
    let mut ctx = setup();
    let owner = Keypair::new();
    let approver = Keypair::new();
    let new_owner = Keypair::new();
    let attacker_owner = Keypair::new();
    ctx.svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    ctx.svm.airdrop(&approver.pubkey(), 1_000_000_000).unwrap();
    ctx.svm.airdrop(&new_owner.pubkey(), 1_000_000_000).unwrap();
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
    let request_data = family_vault::instruction::RequestOwnerRotation { new_owner: new_owner.pubkey(), nonce };
    let ix = Instruction { program_id: family_vault::ID, accounts: request_accounts.to_account_metas(None), data: request_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &approver], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx).unwrap();

    let mut clock = ctx.svm.get_sysvar::<anchor_lang::solana_program::clock::Clock>();
    clock.unix_timestamp += 259_201;
    ctx.svm.set_sysvar(&clock);

    let execute_accounts = family_vault::accounts::ExecuteOwnerRotation { signer: ctx.payer.pubkey(), vault: fixture.vault, rotation };
    let ix = Instruction { program_id: family_vault::ID, accounts: execute_accounts.to_account_metas(None), data: family_vault::instruction::ExecuteOwnerRotation {}.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "first execute_owner_rotation failed: {:?}", result);
    assert_eq!(vault_owner(&ctx, &fixture.vault), new_owner.pubkey());

    // Attempt to resubmit the SAME (already-executed) rotation years later —
    // this must be rejected, not silently hand the vault back to new_owner
    // (harmless here, but if the vault had since rotated again to someone
    // else, replay would hand it back to whoever this stale request named).
    ctx.svm.expire_blockhash();
    let execute_accounts2 = family_vault::accounts::ExecuteOwnerRotation { signer: attacker_owner.pubkey(), vault: fixture.vault, rotation };
    let ix2 = Instruction { program_id: family_vault::ID, accounts: execute_accounts2.to_account_metas(None), data: family_vault::instruction::ExecuteOwnerRotation {}.data() };
    let tx2 = Transaction::new_signed_with_payer(&[ix2], Some(&ctx.payer.pubkey()), &[&ctx.payer, &attacker_owner], ctx.svm.latest_blockhash());
    let result2 = ctx.svm.send_transaction(tx2);
    assert!(result2.is_err(), "expected replayed execute_owner_rotation on an already-executed request to be rejected");
    assert_eq!(vault_owner(&ctx, &fixture.vault), new_owner.pubkey(), "owner must remain the previously-rotated-to owner");
}
