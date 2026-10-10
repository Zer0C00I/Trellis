mod common;

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::{instruction::Instruction, system_program};
use anchor_lang::{InstructionData, ToAccountMetas};
use common::setup;
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::Transaction;

const DAILY_LIMIT: u64 = 50_000_000;

fn init_pool_vault(
    ctx: &mut common::TestCtx,
    owner: &Keypair,
    daily_limit: u64,
) -> (Pubkey, Pubkey, Pubkey) {
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
        daily_limit,
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
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "init_pool_vault failed: {:?}", result);

    (vault, vault_token_account, beneficiary_list)
}

fn add_beneficiary_now(
    ctx: &mut common::TestCtx,
    owner: &Keypair,
    vault: Pubkey,
    beneficiary_list: Pubkey,
    addr: Pubkey,
) {
    let accounts = family_vault::accounts::AddBeneficiary {
        signer: owner.pubkey(),
        vault,
        beneficiary_list,
    };
    let data = family_vault::instruction::AddBeneficiary {
        name: "Beneficiary".to_string(),
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
        &[&ctx.payer, owner],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();
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
fn initializes_a_pool_vault_with_the_given_daily_limit() {
    let mut ctx = setup();
    let owner = Keypair::new();
    ctx.svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    let (vault, _vault_token_account, _beneficiary_list) =
        init_pool_vault(&mut ctx, &owner, DAILY_LIMIT);

    let vault_account = ctx.svm.get_account(&vault).unwrap();
    let vault_data: family_vault::state::Vault =
        anchor_lang::AccountDeserialize::try_deserialize(&mut vault_account.data.as_slice())
            .unwrap();

    assert!(vault_data.is_pool);
    assert_eq!(
        vault_data.daily_limit, DAILY_LIMIT,
        "pool vault daily_limit should be the configured value, not u64::MAX"
    );
    assert_eq!(vault_data.spent_today, 0);
}

#[test]
fn anyone_can_deposit_into_a_pool_vault() {
    let mut ctx = setup();
    let owner = Keypair::new();
    ctx.svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    let (vault, vault_token_account, _beneficiary_list) =
        init_pool_vault(&mut ctx, &owner, DAILY_LIMIT);

    // A depositor who is NOT the vault owner — pool vaults have no owner-only restriction.
    let depositor = Keypair::new();
    ctx.svm.airdrop(&depositor.pubkey(), 1_000_000_000).unwrap();
    let depositor_ata = create_ata(&mut ctx, &depositor.pubkey());
    let mint_ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        &ctx.mint,
        &depositor_ata,
        &ctx.mint_authority.pubkey(),
        &[],
        100_000_000,
    )
    .unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[mint_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &ctx.mint_authority],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();

    let deposit_accounts = family_vault::accounts::Deposit {
        depositor: depositor.pubkey(),
        depositor_token_account: depositor_ata,
        vault_token_account,
        vault,
        token_program: spl_token::id(),
    };
    let deposit_data = family_vault::instruction::Deposit { amount: 80_000_000 };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: deposit_accounts.to_account_metas(None),
        data: deposit_data.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &depositor],
        ctx.svm.latest_blockhash(),
    );
    let result = ctx.svm.send_transaction(tx);
    assert!(
        result.is_ok(),
        "deposit into pool vault by non-owner should succeed: {:?}",
        result
    );

    assert_eq!(token_balance(&ctx, &vault_token_account), 80_000_000);
}

#[test]
fn withdraws_under_daily_limit_and_tracks_spent_today() {
    let mut ctx = setup();
    let owner = Keypair::new();
    ctx.svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    let (vault, vault_token_account, beneficiary_list) =
        init_pool_vault(&mut ctx, &owner, DAILY_LIMIT);

    let mint_ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        &ctx.mint,
        &vault_token_account,
        &ctx.mint_authority.pubkey(),
        &[],
        100_000_000,
    )
    .unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[mint_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &ctx.mint_authority],
        ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();

    let beneficiary = Keypair::new();
    ctx.svm
        .airdrop(&beneficiary.pubkey(), 1_000_000_000)
        .unwrap();
    add_beneficiary_now(
        &mut ctx,
        &owner,
        vault,
        beneficiary_list,
        beneficiary.pubkey(),
    );

    let mut clock = ctx
        .svm
        .get_sysvar::<anchor_lang::solana_program::clock::Clock>();
    clock.unix_timestamp += 172_801;
    ctx.svm.set_sysvar(&clock);

    let beneficiary_ata = create_ata(&mut ctx, &beneficiary.pubkey());

    let accounts = family_vault::accounts::Withdraw {
        signer: owner.pubkey(),
        vault_token_account,
        recipient_token_account: beneficiary_ata,
        vault,
        beneficiary_list,
        token_program: spl_token::id(),
    };
    let data = family_vault::instruction::Withdraw { amount: 20_000_000 };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &owner],
        ctx.svm.latest_blockhash(),
    );
    let result = ctx.svm.send_transaction(tx);
    assert!(
        result.is_ok(),
        "withdraw under daily limit failed: {:?}",
        result
    );

    assert_eq!(token_balance(&ctx, &beneficiary_ata), 20_000_000);

    let vault_account = ctx.svm.get_account(&vault).unwrap();
    let vault_data: family_vault::state::Vault =
        anchor_lang::AccountDeserialize::try_deserialize(&mut vault_account.data.as_slice())
            .unwrap();
    assert_eq!(vault_data.spent_today, 20_000_000);
}

#[test]
fn rejects_withdrawal_that_would_exceed_daily_limit() {
    let mut ctx = setup();
    let owner = Keypair::new();
    ctx.svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    let (vault, vault_token_account, beneficiary_list) =
        init_pool_vault(&mut ctx, &owner, DAILY_LIMIT);

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

    let beneficiary = Keypair::new();
    ctx.svm
        .airdrop(&beneficiary.pubkey(), 1_000_000_000)
        .unwrap();
    add_beneficiary_now(
        &mut ctx,
        &owner,
        vault,
        beneficiary_list,
        beneficiary.pubkey(),
    );

    let mut clock = ctx
        .svm
        .get_sysvar::<anchor_lang::solana_program::clock::Clock>();
    clock.unix_timestamp += 172_801;
    ctx.svm.set_sysvar(&clock);

    let beneficiary_ata = create_ata(&mut ctx, &beneficiary.pubkey());

    // Amount exceeds DAILY_LIMIT (50_000_000) on its own.
    let accounts = family_vault::accounts::Withdraw {
        signer: owner.pubkey(),
        vault_token_account,
        recipient_token_account: beneficiary_ata,
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
        &[&ctx.payer, &owner],
        ctx.svm.latest_blockhash(),
    );
    let result = ctx.svm.send_transaction(tx);
    assert!(
        result.is_err(),
        "expected withdrawal exceeding daily limit to be rejected"
    );

    assert_eq!(
        token_balance(&ctx, &beneficiary_ata),
        0,
        "no funds should have moved on a rejected over-limit withdrawal"
    );
}

#[test]
fn resets_spent_today_after_more_than_a_day_elapses() {
    let mut ctx = setup();
    let owner = Keypair::new();
    ctx.svm.airdrop(&owner.pubkey(), 1_000_000_000).unwrap();
    let (vault, vault_token_account, beneficiary_list) =
        init_pool_vault(&mut ctx, &owner, DAILY_LIMIT);

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

    let beneficiary = Keypair::new();
    ctx.svm
        .airdrop(&beneficiary.pubkey(), 1_000_000_000)
        .unwrap();
    add_beneficiary_now(
        &mut ctx,
        &owner,
        vault,
        beneficiary_list,
        beneficiary.pubkey(),
    );

    let mut clock = ctx
        .svm
        .get_sysvar::<anchor_lang::solana_program::clock::Clock>();
    clock.unix_timestamp += 172_801;
    ctx.svm.set_sysvar(&clock);

    let beneficiary_ata = create_ata(&mut ctx, &beneficiary.pubkey());

    // First withdrawal: spend right up to the daily limit.
    let accounts = family_vault::accounts::Withdraw {
        signer: owner.pubkey(),
        vault_token_account,
        recipient_token_account: beneficiary_ata,
        vault,
        beneficiary_list,
        token_program: spl_token::id(),
    };
    let data = family_vault::instruction::Withdraw {
        amount: DAILY_LIMIT,
    };
    let ix = Instruction {
        program_id: family_vault::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &owner],
        ctx.svm.latest_blockhash(),
    );
    let result = ctx.svm.send_transaction(tx);
    assert!(
        result.is_ok(),
        "first withdrawal up to daily limit should succeed: {:?}",
        result
    );

    let vault_account = ctx.svm.get_account(&vault).unwrap();
    let vault_data: family_vault::state::Vault =
        anchor_lang::AccountDeserialize::try_deserialize(&mut vault_account.data.as_slice())
            .unwrap();
    assert_eq!(vault_data.spent_today, DAILY_LIMIT);

    // Warp the clock forward by more than 86400 seconds (a full day).
    let mut clock = ctx
        .svm
        .get_sysvar::<anchor_lang::solana_program::clock::Clock>();
    clock.unix_timestamp += 86_401;
    ctx.svm.set_sysvar(&clock);

    // A second withdrawal that would have exceeded the previous day's limit
    // (DAILY_LIMIT already spent + this amount > DAILY_LIMIT) must now
    // succeed because the daily counter was lazily reset.
    let accounts = family_vault::accounts::Withdraw {
        signer: owner.pubkey(),
        vault_token_account,
        recipient_token_account: beneficiary_ata,
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
        &[&ctx.payer, &owner],
        ctx.svm.latest_blockhash(),
    );
    let result = ctx.svm.send_transaction(tx);
    assert!(
        result.is_ok(),
        "withdrawal after daily reset should succeed: {:?}",
        result
    );

    let vault_account = ctx.svm.get_account(&vault).unwrap();
    let vault_data: family_vault::state::Vault =
        anchor_lang::AccountDeserialize::try_deserialize(&mut vault_account.data.as_slice())
            .unwrap();
    assert_eq!(
        vault_data.spent_today, 10_000_000,
        "spent_today should have reset to just the new day's withdrawal"
    );

    assert_eq!(
        token_balance(&ctx, &beneficiary_ata),
        DAILY_LIMIT + 10_000_000
    );
}
