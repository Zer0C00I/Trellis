mod common;

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::{instruction::Instruction, system_program};
use anchor_lang::{InstructionData, ToAccountMetas};
use common::setup;
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::Transaction;

fn init_personal_vault_with_funds(
    ctx: &mut common::TestCtx,
    elder: &Keypair,
    amount: u64,
) -> (Pubkey, Pubkey, Pubkey) {
    let (vault, _) = Pubkey::find_program_address(
        &[b"vault", elder.pubkey().as_ref(), b"personal"],
        &family_vault::ID,
    );
    let (beneficiary_list, _) =
        Pubkey::find_program_address(&[b"beneficiaries", vault.as_ref()], &family_vault::ID);
    let vault_token_account =
        spl_associated_token_account::get_associated_token_address(&vault, &ctx.mint);

    let init_accounts = family_vault::accounts::InitializeVault {
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
    let init_data = family_vault::instruction::InitializeVault {
        is_pool: false,
        daily_limit: 0,
    };
    let ix1 = Instruction {
        program_id: family_vault::ID,
        accounts: init_accounts.to_account_metas(None),
        data: init_data.data(),
    };
    let tx1 = Transaction::new_signed_with_payer(
        &[ix1],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, elder],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx1).unwrap();

    // Fund the vault directly via mint_to (simulates a prior deposit for test simplicity).
    let mint_ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        &ctx.mint,
        &vault_token_account,
        &ctx.mint_authority.pubkey(),
        &[],
        amount,
    )
    .unwrap();
    let tx2 = Transaction::new_signed_with_payer(
        &[mint_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &ctx.mint_authority],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx2).unwrap();

    (vault, vault_token_account, beneficiary_list)
}

fn add_beneficiary_now(
    ctx: &mut common::TestCtx,
    elder: &Keypair,
    vault: Pubkey,
    beneficiary_list: Pubkey,
    addr: Pubkey,
) {
    let accounts = family_vault::accounts::AddBeneficiary {
        signer: elder.pubkey(),
        vault,
        beneficiary_list,
    };
    let data = family_vault::instruction::AddBeneficiary {
        name: "Test".to_string(),
        address: addr,
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
}

#[test]
fn rejects_withdrawal_to_unknown_recipient() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();
    let (vault, vault_token_account, beneficiary_list) =
        init_personal_vault_with_funds(&mut ctx, &elder, 100_000_000);

    let stranger = Keypair::new();
    ctx.svm.airdrop(&stranger.pubkey(), 1_000_000_000).unwrap();
    let stranger_ata_ix =
        spl_associated_token_account::instruction::create_associated_token_account(
            &ctx.payer.pubkey(),
            &stranger.pubkey(),
            &ctx.mint,
            &spl_token::id(),
        );
    let tx0 = Transaction::new_signed_with_payer(
        &[stranger_ata_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx0).unwrap();
    let stranger_ata =
        spl_associated_token_account::get_associated_token_address(&stranger.pubkey(), &ctx.mint);

    let accounts = family_vault::accounts::Withdraw {
        signer: elder.pubkey(),
        vault_token_account,
        recipient_token_account: stranger_ata,
        vault,
        beneficiary_list,
        token_program: spl_token::id(),
    };
    let data = family_vault::instruction::Withdraw { amount: 10_000_000 };
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
    assert!(
        result.is_err(),
        "expected withdraw to unknown recipient to fail"
    );
}

#[test]
fn rejects_withdrawal_before_beneficiary_cooldown_elapses() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();
    let (vault, vault_token_account, beneficiary_list) =
        init_personal_vault_with_funds(&mut ctx, &elder, 100_000_000);

    let grandson = Keypair::new();
    ctx.svm.airdrop(&grandson.pubkey(), 1_000_000_000).unwrap();
    add_beneficiary_now(&mut ctx, &elder, vault, beneficiary_list, grandson.pubkey());

    // No clock warp here: the beneficiary was just added, so its 48h cooldown
    // has not elapsed yet. The withdrawal must be rejected with
    // BeneficiaryCooldownActive rather than succeeding.
    let grandson_ata_ix =
        spl_associated_token_account::instruction::create_associated_token_account(
            &ctx.payer.pubkey(),
            &grandson.pubkey(),
            &ctx.mint,
            &spl_token::id(),
        );
    let tx0 = Transaction::new_signed_with_payer(
        &[grandson_ata_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx0).unwrap();
    let grandson_ata =
        spl_associated_token_account::get_associated_token_address(&grandson.pubkey(), &ctx.mint);

    let accounts = family_vault::accounts::Withdraw {
        signer: elder.pubkey(),
        vault_token_account,
        recipient_token_account: grandson_ata,
        vault,
        beneficiary_list,
        token_program: spl_token::id(),
    };
    let data = family_vault::instruction::Withdraw { amount: 10_000_000 };
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
    assert!(
        result.is_err(),
        "expected withdrawal before cooldown elapses to fail"
    );

    let grandson_ata_account = ctx.svm.get_account(&grandson_ata).unwrap();
    let grandson_ata_data: spl_token::state::Account =
        anchor_lang::solana_program::program_pack::Pack::unpack(&grandson_ata_account.data)
            .unwrap();
    assert_eq!(
        grandson_ata_data.amount, 0,
        "no funds should have moved on a rejected cooldown withdrawal"
    );
}

#[test]
fn owner_can_withdraw_unlimited_amount_to_an_active_beneficiary() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();
    let (vault, vault_token_account, beneficiary_list) =
        init_personal_vault_with_funds(&mut ctx, &elder, 100_000_000);

    let grandson = Keypair::new();
    ctx.svm.airdrop(&grandson.pubkey(), 1_000_000_000).unwrap();
    add_beneficiary_now(&mut ctx, &elder, vault, beneficiary_list, grandson.pubkey());

    // NOTE: active_after is now + 48h, so this transaction is expected to be
    // rejected by BeneficiaryCooldownActive in real time. litesvm's clock
    // starts at the host's wall-clock time and does not auto-advance, so to
    // exercise the success path this test warps the clock forward past the
    // cooldown before withdrawing.
    let mut clock = ctx
        .svm
        .get_sysvar::<anchor_lang::solana_program::clock::Clock>();
    clock.unix_timestamp += 172_801;
    ctx.svm.set_sysvar(&clock);

    let grandson_ata_ix =
        spl_associated_token_account::instruction::create_associated_token_account(
            &ctx.payer.pubkey(),
            &grandson.pubkey(),
            &ctx.mint,
            &spl_token::id(),
        );
    let tx0 = Transaction::new_signed_with_payer(
        &[grandson_ata_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx0).unwrap();
    let grandson_ata =
        spl_associated_token_account::get_associated_token_address(&grandson.pubkey(), &ctx.mint);

    let accounts = family_vault::accounts::Withdraw {
        signer: elder.pubkey(),
        vault_token_account,
        recipient_token_account: grandson_ata,
        vault,
        beneficiary_list,
        token_program: spl_token::id(),
    };
    let data = family_vault::instruction::Withdraw { amount: 60_000_000 };
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
    assert!(result.is_ok(), "withdraw failed: {:?}", result);

    let grandson_ata_account = ctx.svm.get_account(&grandson_ata).unwrap();
    let grandson_ata_data: spl_token::state::Account =
        anchor_lang::solana_program::program_pack::Pack::unpack(&grandson_ata_account.data)
            .unwrap();
    assert_eq!(grandson_ata_data.amount, 60_000_000);
}

#[test]
fn rejects_withdrawal_using_a_beneficiary_list_from_a_different_vault() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();
    let (vault, vault_token_account, _beneficiary_list) =
        init_personal_vault_with_funds(&mut ctx, &elder, 100_000_000);

    // A second, unrelated vault whose beneficiary list approves `grandson`.
    let other_elder = Keypair::new();
    ctx.svm
        .airdrop(&other_elder.pubkey(), 1_000_000_000)
        .unwrap();
    let (_other_vault, _other_vault_token_account, other_beneficiary_list) =
        init_personal_vault_with_funds(&mut ctx, &other_elder, 100_000_000);

    let grandson = Keypair::new();
    ctx.svm.airdrop(&grandson.pubkey(), 1_000_000_000).unwrap();
    add_beneficiary_now(
        &mut ctx,
        &other_elder,
        _other_vault,
        other_beneficiary_list,
        grandson.pubkey(),
    );

    let mut clock = ctx
        .svm
        .get_sysvar::<anchor_lang::solana_program::clock::Clock>();
    clock.unix_timestamp += 172_801;
    ctx.svm.set_sysvar(&clock);

    let grandson_ata_ix =
        spl_associated_token_account::instruction::create_associated_token_account(
            &ctx.payer.pubkey(),
            &grandson.pubkey(),
            &ctx.mint,
            &spl_token::id(),
        );
    let tx0 = Transaction::new_signed_with_payer(
        &[grandson_ata_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx0).unwrap();
    let grandson_ata =
        spl_associated_token_account::get_associated_token_address(&grandson.pubkey(), &ctx.mint);

    // Attempt to withdraw from `vault` (elder's vault, which never approved
    // `grandson`) by passing in `other_beneficiary_list` (which belongs to
    // `other_vault` and does approve `grandson`). This must be rejected by
    // the `has_one = vault` constraint on `beneficiary_list` — without it,
    // the closed-address-book anti-scam check could be bypassed entirely by
    // pointing at a different vault's approved list.
    let accounts = family_vault::accounts::Withdraw {
        signer: elder.pubkey(),
        vault_token_account,
        recipient_token_account: grandson_ata,
        vault,
        beneficiary_list: other_beneficiary_list,
        token_program: spl_token::id(),
    };
    let data = family_vault::instruction::Withdraw { amount: 10_000_000 };
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
    assert!(
        result.is_err(),
        "expected withdraw with mismatched beneficiary_list to fail"
    );

    let grandson_ata_account = ctx.svm.get_account(&grandson_ata).unwrap();
    let grandson_ata_data: spl_token::state::Account =
        anchor_lang::solana_program::program_pack::Pack::unpack(&grandson_ata_account.data)
            .unwrap();
    assert_eq!(
        grandson_ata_data.amount, 0,
        "no funds should have moved when beneficiary_list belongs to a different vault"
    );
}

#[test]
fn non_owner_cannot_withdraw() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();
    let (vault, vault_token_account, beneficiary_list) =
        init_personal_vault_with_funds(&mut ctx, &elder, 100_000_000);

    let grandson = Keypair::new();
    ctx.svm.airdrop(&grandson.pubkey(), 1_000_000_000).unwrap();
    add_beneficiary_now(&mut ctx, &elder, vault, beneficiary_list, grandson.pubkey());

    let mut clock = ctx
        .svm
        .get_sysvar::<anchor_lang::solana_program::clock::Clock>();
    clock.unix_timestamp += 172_801;
    ctx.svm.set_sysvar(&clock);

    let grandson_ata_ix =
        spl_associated_token_account::instruction::create_associated_token_account(
            &ctx.payer.pubkey(),
            &grandson.pubkey(),
            &ctx.mint,
            &spl_token::id(),
        );
    let tx0 = Transaction::new_signed_with_payer(
        &[grandson_ata_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx0).unwrap();
    let grandson_ata =
        spl_associated_token_account::get_associated_token_address(&grandson.pubkey(), &ctx.mint);

    // A non-owner keypair attempts to withdraw from a personal vault it doesn't own.
    let attacker = Keypair::new();
    ctx.svm.airdrop(&attacker.pubkey(), 1_000_000_000).unwrap();

    let accounts = family_vault::accounts::Withdraw {
        signer: attacker.pubkey(),
        vault_token_account,
        recipient_token_account: grandson_ata,
        vault,
        beneficiary_list,
        token_program: spl_token::id(),
    };
    let data = family_vault::instruction::Withdraw { amount: 10_000_000 };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &attacker],
        ctx.svm.latest_blockhash(),
    );
    let result = ctx.svm.send_transaction(tx);
    assert!(
        result.is_err(),
        "expected non-owner withdrawal to be rejected"
    );

    let vault_ata_account = ctx.svm.get_account(&vault_token_account).unwrap();
    let vault_ata_data: spl_token::state::Account =
        anchor_lang::solana_program::program_pack::Pack::unpack(&vault_ata_account.data).unwrap();
    assert_eq!(
        vault_ata_data.amount, 100_000_000,
        "vault balance should be unchanged after rejected non-owner withdrawal"
    );
}

#[test]
fn rejects_withdrawal_with_substituted_vault_token_account() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();
    let (vault, _vault_token_account, beneficiary_list) =
        init_personal_vault_with_funds(&mut ctx, &elder, 100_000_000);

    let grandson = Keypair::new();
    ctx.svm.airdrop(&grandson.pubkey(), 1_000_000_000).unwrap();
    add_beneficiary_now(&mut ctx, &elder, vault, beneficiary_list, grandson.pubkey());

    let mut clock = ctx
        .svm
        .get_sysvar::<anchor_lang::solana_program::clock::Clock>();
    clock.unix_timestamp += 172_801;
    ctx.svm.set_sysvar(&clock);

    let grandson_ata_ix =
        spl_associated_token_account::instruction::create_associated_token_account(
            &ctx.payer.pubkey(),
            &grandson.pubkey(),
            &ctx.mint,
            &spl_token::id(),
        );
    let tx0 = Transaction::new_signed_with_payer(
        &[grandson_ata_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx0).unwrap();
    let grandson_ata =
        spl_associated_token_account::get_associated_token_address(&grandson.pubkey(), &ctx.mint);

    // Some OTHER token account (a different vault's), not vault.vault_token_account.
    let other_elder = Keypair::new();
    ctx.svm
        .airdrop(&other_elder.pubkey(), 1_000_000_000)
        .unwrap();
    let (_other_vault, other_vault_token_account, _other_beneficiary_list) =
        init_personal_vault_with_funds(&mut ctx, &other_elder, 50_000_000);

    let accounts = family_vault::accounts::Withdraw {
        signer: elder.pubkey(),
        vault_token_account: other_vault_token_account,
        recipient_token_account: grandson_ata,
        vault,
        beneficiary_list,
        token_program: spl_token::id(),
    };
    let data = family_vault::instruction::Withdraw { amount: 10_000_000 };
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
    assert!(
        result.is_err(),
        "expected withdraw with substituted vault_token_account to be rejected"
    );

    let grandson_ata_account = ctx.svm.get_account(&grandson_ata).unwrap();
    let grandson_ata_data: spl_token::state::Account =
        anchor_lang::solana_program::program_pack::Pack::unpack(&grandson_ata_account.data)
            .unwrap();
    assert_eq!(
        grandson_ata_data.amount, 0,
        "no funds should have moved when vault_token_account is substituted"
    );
}
