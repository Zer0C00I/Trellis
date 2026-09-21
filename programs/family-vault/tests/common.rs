use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::{program_pack::Pack, system_instruction};
use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_transaction::Transaction;
use spl_token::state::Mint as SplMint;

pub const PROGRAM_ID: Pubkey = family_vault::ID;

pub struct TestCtx {
    pub svm: LiteSVM,
    pub payer: Keypair,
    pub mint: Pubkey,
    // Used by most (but not every) test binary to mint test tokens; each
    // `tests/*.rs` file is compiled as its own crate, so a binary that
    // doesn't happen to call mint_to on it sees a (harmless) unused-field
    // warning for this shared helper struct.
    #[allow(dead_code)]
    pub mint_authority: Keypair,
}

pub fn setup() -> TestCtx {
    let mut svm = LiteSVM::new();
    let so_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/deploy/family_vault.so"
    );
    svm.add_program_from_file(PROGRAM_ID, so_path)
        .expect("load family_vault.so — run `cargo build-sbf --manifest-path programs/family-vault/Cargo.toml` from the workspace root first");

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let mint_authority = Keypair::new();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000)
        .unwrap();

    let mint = Keypair::new();
    let rent = svm.minimum_balance_for_rent_exemption(SplMint::LEN);
    let create_mint_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        rent,
        SplMint::LEN as u64,
        &spl_token::id(),
    );
    let init_mint_ix = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint.pubkey(),
        &mint_authority.pubkey(),
        None,
        6,
    )
    .unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_mint_ix, init_mint_ix],
        Some(&payer.pubkey()),
        &[&payer, &mint],
        svm.latest_blockhash(),
    );
    svm.send_transaction(tx).unwrap();

    TestCtx {
        svm,
        payer,
        mint: mint.pubkey(),
        mint_authority,
    }
}
