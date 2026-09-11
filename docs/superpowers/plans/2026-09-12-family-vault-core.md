# Family Vault Core (Anchor program) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and test the on-chain core of the Family Vault program: vault
creation, deposits, beneficiary whitelisting with cooldown, and withdrawals
with the daily-limit path for pool vaults and unlimited path for personal
vaults. This plan produces a deployable, demoable MVP by itself — the
over-limit approval flow and owner-rotation recovery flow are a separate
follow-up plan (`family-vault-approvals`), built on top of this one.

**Architecture:** Single Anchor program (`family_vault`) with one `Vault`
account type shared by personal (unlimited, closed address book) and pool
(daily-limited, closed address book) balances, per the design spec. Fixed-size
arrays for beneficiary/approver lists — no `realloc`. Tests use `litesvm`
(pure Rust, no JS/mocha) loading the compiled `.so` directly.

**Tech Stack:** Rust, Anchor 0.32.1, anchor-spl 0.32.1, solana-cli 2.1.21,
rustc 1.86.0 (verified working combination — see spec Sources), litesvm 0.9.1
for tests, USDC devnet mint for later manual devnet testing.

**Spec:** `docs/superpowers/specs/2026-09-12-family-vault-design.md`

## Global Constraints

- Pin exactly: `anchor-lang = "0.32.1"`, `anchor-spl = "0.32.1"`, Solana CLI
  `2.1.21`, `rustc 1.86.0`. Do not mix with Anchor 1.0.0 toolchain — breaking
  changes (Surfpool/LiteSVM-as-default-template) target a different CLI line.
- Token: USDC devnet mint `4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU` for
  manual/devnet testing later; unit tests use a locally-minted test token.
- Fixed-capacity accounts only: `MAX_BENEFICIARIES = 10`, `MAX_APPROVERS = 5`.
  No `realloc` in this plan (spec: "не в MVP").
- Beneficiary cooldown: 48 hours (`172_800` seconds) between `add_beneficiary`
  and that beneficiary becoming usable in `withdraw`.
- Personal vault (`is_pool = false`) has `daily_limit = u64::MAX` and no
  approval flow — only pool vaults use the daily-limit/approval mechanism.
- No JS/TypeScript tooling. All tests are Rust (`litesvm`), all client code
  (later plans) is Rust.

---

## File Structure

```
solana/
├── Cargo.toml                          # workspace root
├── Anchor.toml
└── programs/
    └── family-vault/
        ├── Cargo.toml
        ├── src/
        │   ├── lib.rs                  # #[program] mod, wires instructions
        │   ├── state.rs                # Vault, BeneficiaryList, TrustedApprovers, PendingWithdrawal
        │   ├── errors.rs                # VaultError
        │   └── instructions/
        │       ├── mod.rs
        │       ├── initialize_vault.rs
        │       ├── deposit.rs
        │       ├── add_beneficiary.rs
        │       └── withdraw.rs
        └── tests/
            ├── common.rs                # shared litesvm test harness
            ├── initialize_vault.rs
            ├── deposit.rs
            ├── add_beneficiary.rs
            └── withdraw.rs
```

---

### Task 1: Workspace scaffold

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `Anchor.toml`
- Create: `programs/family-vault/Cargo.toml`
- Create: `programs/family-vault/src/lib.rs`
- Create: `programs/family-vault/src/state.rs` (empty module for now)
- Create: `programs/family-vault/src/errors.rs` (empty module for now)
- Create: `programs/family-vault/src/instructions/mod.rs` (empty module for now)

**Interfaces:**
- Produces: crate `family_vault` with `declare_id!` and an empty `#[program] mod family_vault { }`, ready for later tasks to add instruction handlers.

- [ ] **Step 1: Create the workspace root `Cargo.toml`**

```toml
[workspace]
members = ["programs/family-vault"]
resolver = "2"
```

- [ ] **Step 2: Create `Anchor.toml`**

```toml
[toolchain]
anchor_version = "0.32.1"
solana_version = "2.1.21"

[features]
resolution = true
skip-lint = false

[programs.localnet]
family_vault = "FVau1t111111111111111111111111111111111111"

[programs.devnet]
family_vault = "FVau1t111111111111111111111111111111111111"

[registry]
url = "https://api.apr.dev"

[provider]
cluster = "devnet"
wallet = "~/.config/solana/id.json"

[scripts]
test = "cargo test --manifest-path programs/family-vault/Cargo.toml -- --test-threads=1"
```

- [ ] **Step 3: Create `programs/family-vault/Cargo.toml`**

```toml
[package]
name = "family-vault"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "lib"]
name = "family_vault"

[features]
no-entrypoint = []
cpi = ["no-entrypoint"]
default = []

[dependencies]
anchor-lang = "0.32.1"
anchor-spl = { version = "0.32.1", features = ["token", "associated_token"] }

[dev-dependencies]
litesvm = "0.9.1"
solana-sdk = "2.1.21"
spl-associated-token-account = "6.0.0"
spl-token = "7.0.0"
```

- [ ] **Step 4: Create `programs/family-vault/src/errors.rs` (empty placeholder module body, filled in Task 3)**

```rust
use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("placeholder — replaced in Task 3")]
    Placeholder,
}
```

- [ ] **Step 5: Create `programs/family-vault/src/state.rs` (empty placeholder, filled in Task 2)**

```rust
// State structs added in Task 2.
```

- [ ] **Step 6: Create `programs/family-vault/src/instructions/mod.rs` (empty placeholder, filled starting Task 4)**

```rust
// Instruction handlers added starting Task 4.
```

- [ ] **Step 7: Create `programs/family-vault/src/lib.rs`**

```rust
use anchor_lang::prelude::*;

pub mod errors;
pub mod state;
pub mod instructions;

declare_id!("FVau1t111111111111111111111111111111111111");

#[program]
pub mod family_vault {
    use super::*;
}
```

- [ ] **Step 8: Verify it builds**

Run: `cd programs/family-vault && cargo build-sbf`
Expected: build succeeds, produces `target/deploy/family_vault.so`

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml Anchor.toml programs/family-vault
git commit -m "chore: scaffold family-vault Anchor workspace"
```

---

### Task 2: State module

**Files:**
- Modify: `programs/family-vault/src/state.rs`

**Interfaces:**
- Produces:
  - `Vault { owner: Pubkey, vault_token_account: Pubkey, daily_limit: u64, spent_today: u64, last_reset_day: i64, is_pool: bool, bump: u8 }`, `Vault::SPACE: usize`
  - `Beneficiary { address: Pubkey, name: [u8; 32], name_len: u8, added_at: i64, active_after: i64 }` (Default, Copy, Clone)
  - `BeneficiaryList { vault: Pubkey, count: u8, entries: [Beneficiary; MAX_BENEFICIARIES] }`, `BeneficiaryList::SPACE: usize`
  - Constants: `MAX_BENEFICIARIES: usize = 10`, `MAX_APPROVERS: usize = 5`, `BENEFICIARY_NAME_LEN: usize = 32`, `BENEFICIARY_COOLDOWN_SECS: i64 = 172_800`

- [ ] **Step 1: Write `state.rs`**

```rust
use anchor_lang::prelude::*;

pub const MAX_BENEFICIARIES: usize = 10;
pub const MAX_APPROVERS: usize = 5;
pub const BENEFICIARY_NAME_LEN: usize = 32;
pub const BENEFICIARY_COOLDOWN_SECS: i64 = 172_800; // 48h

#[account]
pub struct Vault {
    pub owner: Pubkey,
    pub vault_token_account: Pubkey,
    pub daily_limit: u64,
    pub spent_today: u64,
    pub last_reset_day: i64,
    pub is_pool: bool,
    pub bump: u8,
}

impl Vault {
    pub const SPACE: usize = 8 // discriminator
        + 32 // owner
        + 32 // vault_token_account
        + 8  // daily_limit
        + 8  // spent_today
        + 8  // last_reset_day
        + 1  // is_pool
        + 1; // bump
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct Beneficiary {
    pub address: Pubkey,
    pub name: [u8; BENEFICIARY_NAME_LEN],
    pub name_len: u8,
    pub added_at: i64,
    pub active_after: i64,
}

impl Default for Beneficiary {
    fn default() -> Self {
        Beneficiary {
            address: Pubkey::default(),
            name: [0u8; BENEFICIARY_NAME_LEN],
            name_len: 0,
            added_at: 0,
            active_after: 0,
        }
    }
}

#[account]
pub struct BeneficiaryList {
    pub vault: Pubkey,
    pub count: u8,
    pub entries: [Beneficiary; MAX_BENEFICIARIES],
}

impl BeneficiaryList {
    pub const SPACE: usize = 8 // discriminator
        + 32 // vault
        + 1  // count
        + (32 + BENEFICIARY_NAME_LEN + 1 + 8 + 8) * MAX_BENEFICIARIES;
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd programs/family-vault && cargo build-sbf`
Expected: build succeeds

- [ ] **Step 3: Commit**

```bash
git add programs/family-vault/src/state.rs
git commit -m "feat: add Vault and BeneficiaryList state accounts"
```

---

### Task 3: Errors module

**Files:**
- Modify: `programs/family-vault/src/errors.rs`

**Interfaces:**
- Produces: `VaultError` variants used by later tasks: `NotVaultOwner`, `BeneficiaryListFull`, `BeneficiaryAlreadyExists`, `NameTooLong`, `UnknownBeneficiary`, `BeneficiaryCooldownActive`, `AmountOverflow`.

- [ ] **Step 1: Write `errors.rs`**

```rust
use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Only the vault owner may deposit into a personal vault")]
    NotVaultOwner,
    #[msg("Beneficiary list is full")]
    BeneficiaryListFull,
    #[msg("Beneficiary already exists")]
    BeneficiaryAlreadyExists,
    #[msg("Beneficiary name exceeds maximum length")]
    NameTooLong,
    #[msg("Recipient is not an approved beneficiary")]
    UnknownBeneficiary,
    #[msg("Beneficiary cooldown has not elapsed yet")]
    BeneficiaryCooldownActive,
    #[msg("Amount overflowed while adding to spent_today")]
    AmountOverflow,
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd programs/family-vault && cargo build-sbf`
Expected: build succeeds (unused-variant warnings are fine at this stage)

- [ ] **Step 3: Commit**

```bash
git add programs/family-vault/src/errors.rs
git commit -m "feat: add VaultError variants"
```

---

### Task 4: `initialize_vault` instruction

**Files:**
- Create: `programs/family-vault/src/instructions/initialize_vault.rs`
- Modify: `programs/family-vault/src/instructions/mod.rs`
- Modify: `programs/family-vault/src/lib.rs`
- Create: `programs/family-vault/tests/common.rs`
- Create: `programs/family-vault/tests/initialize_vault.rs`

**Interfaces:**
- Consumes: `state::{Vault, BeneficiaryList}`
- Produces: `family_vault::instruction::InitializeVault { is_pool: bool, daily_limit: u64 }` (Anchor-generated), `family_vault::accounts::InitializeVault` (Anchor-generated ToAccountMetas), handler `initialize_vault(ctx, is_pool, daily_limit)`

- [ ] **Step 1: Write the instruction handler**

Create `programs/family-vault/src/instructions/initialize_vault.rs`:

```rust
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};
use anchor_spl::associated_token::AssociatedToken;

use crate::state::*;

pub fn initialize_vault(ctx: Context<InitializeVault>, is_pool: bool, daily_limit: u64) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    vault.owner = ctx.accounts.owner.key();
    vault.vault_token_account = ctx.accounts.vault_token_account.key();
    vault.daily_limit = if is_pool { daily_limit } else { u64::MAX };
    vault.spent_today = 0;
    vault.last_reset_day = 0;
    vault.is_pool = is_pool;
    vault.bump = ctx.bumps.vault;

    let list = &mut ctx.accounts.beneficiary_list;
    list.vault = vault.key();
    list.count = 0;

    Ok(())
}

#[derive(Accounts)]
#[instruction(is_pool: bool)]
pub struct InitializeVault<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: only used to derive seeds; not read or written directly here
    pub owner: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        space = Vault::SPACE,
        seeds = [b"vault", owner.key().as_ref(), if is_pool { b"pool".as_ref() } else { b"personal".as_ref() }],
        bump
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        init,
        payer = payer,
        space = BeneficiaryList::SPACE,
        seeds = [b"beneficiaries", vault.key().as_ref()],
        bump
    )]
    pub beneficiary_list: Account<'info, BeneficiaryList>,

    pub mint: Account<'info, Mint>,

    #[account(
        init,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = vault,
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
```

- [ ] **Step 2: Wire it into `instructions/mod.rs`**

```rust
pub mod initialize_vault;
pub use initialize_vault::*;
```

- [ ] **Step 3: Wire it into `lib.rs`**

```rust
use anchor_lang::prelude::*;

pub mod errors;
pub mod state;
pub mod instructions;

use instructions::*;

declare_id!("FVau1t111111111111111111111111111111111111");

#[program]
pub mod family_vault {
    use super::*;

    pub fn initialize_vault(ctx: Context<InitializeVault>, is_pool: bool, daily_limit: u64) -> Result<()> {
        instructions::initialize_vault(ctx, is_pool, daily_limit)
    }
}
```

- [ ] **Step 4: Build**

Run: `cd programs/family-vault && cargo build-sbf`
Expected: build succeeds, `target/deploy/family_vault.so` updated

- [ ] **Step 5: Write the shared litesvm test harness**

Create `programs/family-vault/tests/common.rs`:

```rust
use litesvm::LiteSVM;
use solana_sdk::{
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_token::state::Mint as SplMint;

pub const PROGRAM_ID: Pubkey = family_vault::ID;

pub struct TestCtx {
    pub svm: LiteSVM,
    pub payer: Keypair,
    pub mint: Pubkey,
    pub mint_authority: Keypair,
}

pub fn setup() -> TestCtx {
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(PROGRAM_ID, "target/deploy/family_vault.so")
        .expect("load family_vault.so — run `cargo build-sbf` first");

    let payer = Keypair::new();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    let mint_authority = Keypair::new();
    svm.airdrop(&mint_authority.pubkey(), 1_000_000_000).unwrap();

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

    TestCtx { svm, payer, mint: mint.pubkey(), mint_authority }
}
```

- [ ] **Step 6: Write the failing test**

Create `programs/family-vault/tests/initialize_vault.rs`:

```rust
mod common;

use anchor_lang::{InstructionData, ToAccountMetas};
use common::setup;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};

#[test]
fn initializes_a_personal_vault_with_unlimited_daily_limit() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();

    let (vault, _) = Pubkey::find_program_address(
        &[b"vault", elder.pubkey().as_ref(), b"personal"],
        &family_vault::ID,
    );
    let (beneficiary_list, _) =
        Pubkey::find_program_address(&[b"beneficiaries", vault.as_ref()], &family_vault::ID);
    let vault_token_account = spl_associated_token_account::get_associated_token_address(
        &vault,
        &ctx.mint,
    );

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
        &[&ctx.payer],
        ctx.svm.latest_blockhash(),
    );
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "initialize_vault failed: {:?}", result);

    let vault_account = ctx.svm.get_account(&vault).unwrap();
    let vault_data: family_vault::state::Vault =
        anchor_lang::AccountDeserialize::try_deserialize(&mut vault_account.data.as_slice())
            .unwrap();

    assert_eq!(vault_data.owner, elder.pubkey());
    assert_eq!(vault_data.is_pool, false);
    assert_eq!(vault_data.daily_limit, u64::MAX);
    assert_eq!(vault_data.spent_today, 0);
}
```

- [ ] **Step 7: Run the test**

Run: `cd programs/family-vault && cargo build-sbf && cargo test --test initialize_vault -- --nocapture`
Expected: PASS — `initializes_a_personal_vault_with_unlimited_daily_limit ... ok`

- [ ] **Step 8: Commit**

```bash
git add programs/family-vault/src programs/family-vault/tests programs/family-vault/Cargo.toml
git commit -m "feat: implement initialize_vault instruction with litesvm test"
```

---

### Task 5: `deposit` instruction

**Files:**
- Create: `programs/family-vault/src/instructions/deposit.rs`
- Modify: `programs/family-vault/src/instructions/mod.rs`
- Modify: `programs/family-vault/src/lib.rs`
- Create: `programs/family-vault/tests/deposit.rs`

**Interfaces:**
- Consumes: `state::Vault`, `errors::VaultError::NotVaultOwner`, `common::setup` (Task 4)
- Produces: `family_vault::instruction::Deposit { amount: u64 }`, `family_vault::accounts::Deposit`, handler `deposit(ctx, amount)`

- [ ] **Step 1: Write the instruction handler**

Create `programs/family-vault/src/instructions/deposit.rs`:

```rust
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::errors::VaultError;
use crate::state::Vault;

pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let vault = &ctx.accounts.vault;
    if !vault.is_pool {
        require_keys_eq!(
            ctx.accounts.depositor.key(),
            vault.owner,
            VaultError::NotVaultOwner
        );
    }

    let cpi_accounts = Transfer {
        from: ctx.accounts.depositor_token_account.to_account_info(),
        to: ctx.accounts.vault_token_account.to_account_info(),
        authority: ctx.accounts.depositor.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token::transfer(cpi_ctx, amount)
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub depositor: Signer<'info>,

    #[account(mut)]
    pub depositor_token_account: Account<'info, TokenAccount>,

    #[account(mut, address = vault.vault_token_account)]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub vault: Account<'info, Vault>,

    pub token_program: Program<'info, Token>,
}
```

- [ ] **Step 2: Wire into `instructions/mod.rs`**

```rust
pub mod initialize_vault;
pub use initialize_vault::*;
pub mod deposit;
pub use deposit::*;
```

- [ ] **Step 3: Wire into `lib.rs`**

```rust
pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    instructions::deposit(ctx, amount)
}
```

(add inside the existing `#[program] pub mod family_vault { ... }` block, after `initialize_vault`)

- [ ] **Step 4: Build**

Run: `cd programs/family-vault && cargo build-sbf`
Expected: build succeeds

- [ ] **Step 5: Write the failing test**

Create `programs/family-vault/tests/deposit.rs`:

```rust
mod common;

use anchor_lang::{InstructionData, ToAccountMetas};
use common::setup;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};

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
    let data = family_vault::instruction::InitializeVault { is_pool: false, daily_limit: 0 };
    let ix = Instruction { program_id: family_vault::ID, accounts: accounts.to_account_metas(None), data: data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx).unwrap();

    (vault, vault_token_account)
}

#[test]
fn owner_can_deposit_into_own_personal_vault() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();
    let (vault, vault_token_account) = init_personal_vault(&mut ctx, &elder);

    // Create elder's own token account and mint 100 test tokens (6 decimals) into it.
    let elder_ata = spl_associated_token_account::get_associated_token_address(&elder.pubkey(), &ctx.mint);
    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &ctx.payer.pubkey(), &elder.pubkey(), &ctx.mint, &spl_token::id(),
    );
    let mint_to_ix = spl_token::instruction::mint_to(
        &spl_token::id(), &ctx.mint, &elder_ata, &ctx.mint_authority.pubkey(), &[], 100_000_000,
    ).unwrap();
    let tx = Transaction::new_signed_with_payer(
        &[create_ata_ix, mint_to_ix], Some(&ctx.payer.pubkey()),
        &[&ctx.payer, &ctx.mint_authority], ctx.svm.latest_blockhash(),
    );
    ctx.svm.send_transaction(tx).unwrap();

    let deposit_accounts = family_vault::accounts::Deposit {
        depositor: elder.pubkey(),
        depositor_token_account: elder_ata,
        vault_token_account,
        vault,
        token_program: spl_token::id(),
    };
    let deposit_data = family_vault::instruction::Deposit { amount: 40_000_000 };
    let ix = Instruction { program_id: family_vault::ID, accounts: deposit_accounts.to_account_metas(None), data: deposit_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &elder], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "deposit failed: {:?}", result);

    let vault_ata_account = ctx.svm.get_account(&vault_token_account).unwrap();
    let vault_ata_data: spl_token::state::Account =
        anchor_lang::solana_program::program_pack::Pack::unpack(&vault_ata_account.data).unwrap();
    assert_eq!(vault_ata_data.amount, 40_000_000);
}
```

- [ ] **Step 6: Run the test**

Run: `cd programs/family-vault && cargo build-sbf && cargo test --test deposit -- --nocapture`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add programs/family-vault/src programs/family-vault/tests
git commit -m "feat: implement deposit instruction with owner-only check for personal vaults"
```

---

### Task 6: `add_beneficiary` instruction

**Files:**
- Create: `programs/family-vault/src/instructions/add_beneficiary.rs`
- Modify: `programs/family-vault/src/instructions/mod.rs`
- Modify: `programs/family-vault/src/lib.rs`
- Create: `programs/family-vault/tests/add_beneficiary.rs`

**Interfaces:**
- Consumes: `state::{Vault, BeneficiaryList, Beneficiary, MAX_BENEFICIARIES, BENEFICIARY_NAME_LEN, BENEFICIARY_COOLDOWN_SECS}`, `errors::VaultError::{BeneficiaryListFull, BeneficiaryAlreadyExists, NameTooLong}`
- Produces: `family_vault::instruction::AddBeneficiary { name: String, address: Pubkey }`, `family_vault::accounts::AddBeneficiary`, handler `add_beneficiary(ctx, name, address)`

- [ ] **Step 1: Write the instruction handler**

Create `programs/family-vault/src/instructions/add_beneficiary.rs`:

```rust
use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::state::{Beneficiary, BeneficiaryList, Vault, BENEFICIARY_COOLDOWN_SECS, BENEFICIARY_NAME_LEN, MAX_BENEFICIARIES};

pub fn add_beneficiary(ctx: Context<AddBeneficiary>, name: String, address: Pubkey) -> Result<()> {
    require!(name.len() <= BENEFICIARY_NAME_LEN, VaultError::NameTooLong);

    let list = &mut ctx.accounts.beneficiary_list;
    require!((list.count as usize) < MAX_BENEFICIARIES, VaultError::BeneficiaryListFull);

    for i in 0..(list.count as usize) {
        require!(list.entries[i].address != address, VaultError::BeneficiaryAlreadyExists);
    }

    let now = Clock::get()?.unix_timestamp;
    let mut name_bytes = [0u8; BENEFICIARY_NAME_LEN];
    let src = name.as_bytes();
    name_bytes[..src.len()].copy_from_slice(src);

    let idx = list.count as usize;
    list.entries[idx] = Beneficiary {
        address,
        name: name_bytes,
        name_len: src.len() as u8,
        added_at: now,
        active_after: now + BENEFICIARY_COOLDOWN_SECS,
    };
    list.count += 1;

    Ok(())
}

#[derive(Accounts)]
pub struct AddBeneficiary<'info> {
    #[account(constraint = signer.key() == vault.owner)]
    pub signer: Signer<'info>,

    pub vault: Account<'info, Vault>,

    #[account(mut, has_one = vault)]
    pub beneficiary_list: Account<'info, BeneficiaryList>,
}
```

Note: `has_one = vault` requires `BeneficiaryList` to have a field literally
named `vault` (it does) matching the `vault` account passed into the
instruction — this is Anchor's built-in relationship check, not custom code.

- [ ] **Step 2: Wire into `instructions/mod.rs`**

```rust
pub mod add_beneficiary;
pub use add_beneficiary::*;
```

- [ ] **Step 3: Wire into `lib.rs`**

```rust
pub fn add_beneficiary(ctx: Context<AddBeneficiary>, name: String, address: Pubkey) -> Result<()> {
    instructions::add_beneficiary(ctx, name, address)
}
```

- [ ] **Step 4: Build**

Run: `cd programs/family-vault && cargo build-sbf`
Expected: build succeeds

- [ ] **Step 5: Write the failing test**

Create `programs/family-vault/tests/add_beneficiary.rs`:

```rust
mod common;

use anchor_lang::{InstructionData, ToAccountMetas};
use common::setup;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};

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
```

- [ ] **Step 6: Run the test**

Run: `cd programs/family-vault && cargo build-sbf && cargo test --test add_beneficiary -- --nocapture`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add programs/family-vault/src programs/family-vault/tests
git commit -m "feat: implement add_beneficiary with 48h cooldown"
```

---

### Task 7: `withdraw` instruction (direct path — under limit / unlimited personal)

**Files:**
- Create: `programs/family-vault/src/instructions/withdraw.rs`
- Modify: `programs/family-vault/src/instructions/mod.rs`
- Modify: `programs/family-vault/src/lib.rs`
- Create: `programs/family-vault/tests/withdraw.rs`

**Interfaces:**
- Consumes: `state::{Vault, BeneficiaryList}`, `errors::VaultError::{UnknownBeneficiary, BeneficiaryCooldownActive, AmountOverflow}`
- Produces: `family_vault::instruction::Withdraw { amount: u64 }`, `family_vault::accounts::Withdraw`, handler `withdraw(ctx, amount)`

This task covers only the path where the withdrawal does not exceed
`daily_limit` (always true for personal vaults, since `daily_limit =
u64::MAX`). The over-limit → `PendingWithdrawal` path is built in the
`family-vault-approvals` follow-up plan, once this direct path is proven.

- [ ] **Step 1: Write the instruction handler**

Create `programs/family-vault/src/instructions/withdraw.rs`:

```rust
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::errors::VaultError;
use crate::state::{BeneficiaryList, Vault};

pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    let to_key = ctx.accounts.recipient_token_account.owner;
    let list = &ctx.accounts.beneficiary_list;
    let clock = Clock::get()?;

    let mut found = false;
    for i in 0..(list.count as usize) {
        let b = &list.entries[i];
        if b.address == to_key {
            require!(clock.unix_timestamp >= b.active_after, VaultError::BeneficiaryCooldownActive);
            found = true;
            break;
        }
    }
    require!(found, VaultError::UnknownBeneficiary);

    let vault = &mut ctx.accounts.vault;
    let today = clock.unix_timestamp / 86_400;
    if vault.is_pool {
        if vault.last_reset_day != today {
            vault.spent_today = 0;
            vault.last_reset_day = today;
        }
        let new_spent = vault
            .spent_today
            .checked_add(amount)
            .ok_or(VaultError::AmountOverflow)?;
        require!(new_spent <= vault.daily_limit, VaultError::AmountOverflow); // over-limit path: family-vault-approvals plan
        vault.spent_today = new_spent;
    }

    let owner_key = vault.owner;
    let seed_kind: &[u8] = if vault.is_pool { b"pool" } else { b"personal" };
    let bump = vault.bump;
    let vault_seeds: &[&[u8]] = &[b"vault", owner_key.as_ref(), seed_kind, &[bump]];
    let signer_seeds: &[&[&[u8]]] = &[vault_seeds];

    let cpi_accounts = Transfer {
        from: ctx.accounts.vault_token_account.to_account_info(),
        to: ctx.accounts.recipient_token_account.to_account_info(),
        authority: vault.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    token::transfer(cpi_ctx, amount)
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(constraint = signer.key() == vault.owner)]
    pub signer: Signer<'info>,

    #[account(mut, address = vault.vault_token_account)]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub recipient_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub vault: Account<'info, Vault>,

    pub beneficiary_list: Account<'info, BeneficiaryList>,

    pub token_program: Program<'info, Token>,
}
```

- [ ] **Step 2: Wire into `instructions/mod.rs`**

```rust
pub mod withdraw;
pub use withdraw::*;
```

- [ ] **Step 3: Wire into `lib.rs`**

```rust
pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    instructions::withdraw(ctx, amount)
}
```

- [ ] **Step 4: Build**

Run: `cd programs/family-vault && cargo build-sbf`
Expected: build succeeds

- [ ] **Step 5: Write the failing tests (happy path + rejection paths)**

Create `programs/family-vault/tests/withdraw.rs`:

```rust
mod common;

use anchor_lang::{InstructionData, ToAccountMetas};
use common::setup;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};

fn init_personal_vault_with_funds(ctx: &mut common::TestCtx, elder: &Keypair, amount: u64) -> (Pubkey, Pubkey, Pubkey) {
    let (vault, _) = Pubkey::find_program_address(&[b"vault", elder.pubkey().as_ref(), b"personal"], &family_vault::ID);
    let (beneficiary_list, _) = Pubkey::find_program_address(&[b"beneficiaries", vault.as_ref()], &family_vault::ID);
    let vault_token_account = spl_associated_token_account::get_associated_token_address(&vault, &ctx.mint);

    let init_accounts = family_vault::accounts::InitializeVault {
        payer: ctx.payer.pubkey(), owner: elder.pubkey(), vault, beneficiary_list,
        mint: ctx.mint, vault_token_account, token_program: spl_token::id(),
        associated_token_program: spl_associated_token_account::id(), system_program: system_program::id(),
    };
    let init_data = family_vault::instruction::InitializeVault { is_pool: false, daily_limit: 0 };
    let ix1 = Instruction { program_id: family_vault::ID, accounts: init_accounts.to_account_metas(None), data: init_data.data() };
    let tx1 = Transaction::new_signed_with_payer(&[ix1], Some(&ctx.payer.pubkey()), &[&ctx.payer], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx1).unwrap();

    // Fund the vault directly via mint_to (simulates a prior deposit for test simplicity).
    let mint_ix = spl_token::instruction::mint_to(
        &spl_token::id(), &ctx.mint, &vault_token_account, &ctx.mint_authority.pubkey(), &[], amount,
    ).unwrap();
    let tx2 = Transaction::new_signed_with_payer(&[mint_ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &ctx.mint_authority], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx2).unwrap();

    (vault, vault_token_account, beneficiary_list)
}

fn add_beneficiary_now(ctx: &mut common::TestCtx, elder: &Keypair, vault: Pubkey, beneficiary_list: Pubkey, addr: Pubkey) {
    let accounts = family_vault::accounts::AddBeneficiary { signer: elder.pubkey(), vault, beneficiary_list };
    let data = family_vault::instruction::AddBeneficiary { name: "Test".to_string(), address: addr };
    let ix = Instruction { program_id: family_vault::ID, accounts: accounts.to_account_metas(None), data: data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, elder], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx).unwrap();
}

#[test]
fn rejects_withdrawal_to_unknown_recipient() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();
    let (vault, vault_token_account, beneficiary_list) = init_personal_vault_with_funds(&mut ctx, &elder, 100_000_000);

    let stranger = Keypair::new();
    ctx.svm.airdrop(&stranger.pubkey(), 1_000_000_000).unwrap();
    let stranger_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &ctx.payer.pubkey(), &stranger.pubkey(), &ctx.mint, &spl_token::id(),
    );
    let tx0 = Transaction::new_signed_with_payer(&[stranger_ata_ix], Some(&ctx.payer.pubkey()), &[&ctx.payer], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx0).unwrap();
    let stranger_ata = spl_associated_token_account::get_associated_token_address(&stranger.pubkey(), &ctx.mint);

    let accounts = family_vault::accounts::Withdraw {
        signer: elder.pubkey(), vault_token_account, recipient_token_account: stranger_ata,
        vault, beneficiary_list, token_program: spl_token::id(),
    };
    let data = family_vault::instruction::Withdraw { amount: 10_000_000 };
    let ix = Instruction { program_id: family_vault::ID, accounts: accounts.to_account_metas(None), data: data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &elder], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_err(), "expected withdraw to unknown recipient to fail");
}

#[test]
fn owner_can_withdraw_unlimited_amount_to_an_active_beneficiary() {
    let mut ctx = setup();
    let elder = Keypair::new();
    ctx.svm.airdrop(&elder.pubkey(), 1_000_000_000).unwrap();
    let (vault, vault_token_account, beneficiary_list) = init_personal_vault_with_funds(&mut ctx, &elder, 100_000_000);

    let grandson = Keypair::new();
    ctx.svm.airdrop(&grandson.pubkey(), 1_000_000_000).unwrap();
    add_beneficiary_now(&mut ctx, &elder, vault, beneficiary_list, grandson.pubkey());

    // NOTE: active_after is now + 48h, so this transaction is expected to be
    // rejected by BeneficiaryCooldownActive in real time. litesvm's clock
    // starts at the host's wall-clock time and does not auto-advance, so to
    // exercise the success path this test warps the clock forward past the
    // cooldown before withdrawing.
    let mut clock = ctx.svm.get_sysvar::<solana_sdk::clock::Clock>();
    clock.unix_timestamp += 172_801;
    ctx.svm.set_sysvar(&clock);

    let grandson_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &ctx.payer.pubkey(), &grandson.pubkey(), &ctx.mint, &spl_token::id(),
    );
    let tx0 = Transaction::new_signed_with_payer(&[grandson_ata_ix], Some(&ctx.payer.pubkey()), &[&ctx.payer], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx0).unwrap();
    let grandson_ata = spl_associated_token_account::get_associated_token_address(&grandson.pubkey(), &ctx.mint);

    let accounts = family_vault::accounts::Withdraw {
        signer: elder.pubkey(), vault_token_account, recipient_token_account: grandson_ata,
        vault, beneficiary_list, token_program: spl_token::id(),
    };
    let data = family_vault::instruction::Withdraw { amount: 60_000_000 };
    let ix = Instruction { program_id: family_vault::ID, accounts: accounts.to_account_metas(None), data: data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &elder], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "withdraw failed: {:?}", result);

    let grandson_ata_account = ctx.svm.get_account(&grandson_ata).unwrap();
    let grandson_ata_data: spl_token::state::Account =
        anchor_lang::solana_program::program_pack::Pack::unpack(&grandson_ata_account.data).unwrap();
    assert_eq!(grandson_ata_data.amount, 60_000_000);
}
```

- [ ] **Step 6: Run the tests**

Run: `cd programs/family-vault && cargo build-sbf && cargo test --test withdraw -- --nocapture`
Expected: both tests PASS

- [ ] **Step 7: Commit**

```bash
git add programs/family-vault/src programs/family-vault/tests
git commit -m "feat: implement withdraw instruction (direct path, closed address book + cooldown enforced)"
```

---

## Follow-up plan (not in this plan)

`docs/superpowers/plans/<next-date>-family-vault-approvals.md` will add, on
top of this working core:
- `PendingWithdrawal` account + over-limit branch of `withdraw` (currently
  rejects with `AmountOverflow` instead of queuing — Task 7 leaves this as
  the explicit boundary)
- `TrustedApprovers` account + `add_trusted_approver` + `approve_withdrawal`
- `PendingRotation` account + `request_owner_rotation` /
  `execute_owner_rotation` / `cancel_owner_rotation` (recovery flow)
