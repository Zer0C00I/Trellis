# Family Vault Approvals & Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the two on-chain mechanisms deferred from `family-vault-core`:
approvals for pool-vault withdrawals that exceed the daily limit, and
timelocked owner-rotation recovery. Both are built as new, additive
instructions/accounts — the existing `initialize_vault`, `deposit`,
`add_beneficiary`, and `withdraw` instructions are **not modified**.

**Architecture:** `TrustedApprovers` (fixed-capacity list, same pattern as
`BeneficiaryList`) gates two independent flows: (1) `request_withdrawal` +
`approve_withdrawal` for pool-vault amounts over `daily_limit`, and (2)
`request_owner_rotation` + `execute_owner_rotation` / `cancel_owner_rotation`
for device-loss recovery, gated by a 72h timelock.

**Deliberate deviation from the design spec's literal text:** the spec
describes `withdraw()` itself deciding whether to transfer directly or
queue a `PendingWithdrawal`. This plan instead adds a **separate**
`request_withdrawal` instruction instead of modifying `withdraw`. Reason:
`withdraw` already shipped with 22 passing, reviewed tests across three
call sites (`tests/withdraw.rs`, `tests/pool_vault.rs`); making it
conditionally create an extra account within one instruction would force
every existing call site to pass a new `nonce` argument and an `init`'d
`pending_withdrawal` account even on the (overwhelmingly common) under-limit
path, for no functional benefit. A separate explicit instruction per
intent is also more idiomatic Anchor than one instruction with two
runtime-dependent account-shapes. The kiosk client (a later plan) reads
`Vault.daily_limit`/`spent_today` before deciding which instruction to
call — this is a client-side decision, not an on-chain ambiguity.

**Tech Stack:** Same as `family-vault-core`: Anchor 1.2.0, anchor-spl
1.2.0, litesvm 0.10.0, granular solana-* crates, Solana CLI 3.1.10.

**Spec:** `docs/superpowers/specs/2026-09-12-family-vault-design.md`

## Global Constraints

- Do not modify `initialize_vault.rs`, `deposit.rs`, `add_beneficiary.rs`,
  `withdraw.rs`, or any existing test file. All work in this plan is
  additive: new state structs, new instructions, new test files.
- Reuse the exact dependency pins already in `programs/family-vault/Cargo.toml`
  — do not add or change any dependency version.
- Approver list capacity: `MAX_APPROVERS = 5` (already defined in `state.rs`
  from the core plan — do not redefine it, just add to the same file).
- Owner-rotation timelock: 72 hours (`259_200` seconds).
- `request_withdrawal` is only valid for pool vaults (`vault.is_pool == true`)
  — personal vaults have no amount limit, so there is nothing to queue.
- All PDAs that need a per-call nonce (`request_withdrawal`,
  `request_owner_rotation`) take an explicit `nonce: u64` instruction
  argument supplied by the caller (e.g. current unix timestamp or a random
  value) — this lets multiple pending requests coexist per vault.
- No JS/TypeScript tooling. All tests are Rust (`litesvm`).

---

## File Structure

```
programs/family-vault/
├── src/
│   ├── state.rs                       # MODIFY: add TrustedApprovers, PendingWithdrawal, PendingRotation, ROTATION_TIMELOCK_SECS
│   ├── errors.rs                      # MODIFY: add new VaultError variants
│   ├── lib.rs                         # MODIFY: wire new instructions
│   └── instructions/
│       ├── mod.rs                     # MODIFY: export new modules
│       ├── initialize_approvers.rs    # CREATE
│       ├── add_trusted_approver.rs    # CREATE
│       ├── request_withdrawal.rs      # CREATE
│       ├── approve_withdrawal.rs      # CREATE
│       ├── request_owner_rotation.rs  # CREATE
│       ├── cancel_owner_rotation.rs   # CREATE
│       └── execute_owner_rotation.rs  # CREATE
└── tests/
    ├── trusted_approvers.rs           # CREATE
    ├── withdrawal_approval.rs         # CREATE
    └── owner_rotation.rs              # CREATE
```

---

### Task 1: `TrustedApprovers` state + `initialize_approvers` + `add_trusted_approver`

**Files:**
- Modify: `programs/family-vault/src/state.rs`
- Modify: `programs/family-vault/src/errors.rs`
- Create: `programs/family-vault/src/instructions/initialize_approvers.rs`
- Create: `programs/family-vault/src/instructions/add_trusted_approver.rs`
- Modify: `programs/family-vault/src/instructions/mod.rs`
- Modify: `programs/family-vault/src/lib.rs`
- Create: `programs/family-vault/tests/trusted_approvers.rs`

**Interfaces:**
- Consumes: `state::{Vault, MAX_APPROVERS}` (both already exist), `errors::VaultError::NotVaultOwner`
- Produces: `state::TrustedApprovers { vault: Pubkey, count: u8, entries: [Pubkey; MAX_APPROVERS] }`, `errors::VaultError::{ApproverListFull, ApproverAlreadyExists}`, `family_vault::instruction::{InitializeApprovers, AddTrustedApprover}`, `family_vault::accounts::{InitializeApprovers, AddTrustedApprover}`

- [ ] **Step 1: Add `TrustedApprovers` to `state.rs`**

Append to the existing `programs/family-vault/src/state.rs` (do not remove
anything already there):

```rust

#[account]
pub struct TrustedApprovers {
    pub vault: Pubkey,
    pub count: u8,
    pub entries: [Pubkey; MAX_APPROVERS],
}

impl TrustedApprovers {
    pub const SPACE: usize = 8 // discriminator
        + 32 // vault
        + 1  // count
        + 32 * MAX_APPROVERS;
}
```

- [ ] **Step 2: Add new error variants to `errors.rs`**

Append two new variants inside the existing `#[error_code] pub enum
VaultError { ... }` block (do not remove existing variants):

```rust
    #[msg("Trusted approver list is full")]
    ApproverListFull,
    #[msg("Address is already a trusted approver")]
    ApproverAlreadyExists,
```

- [ ] **Step 3: Write `initialize_approvers.rs`**

Create `programs/family-vault/src/instructions/initialize_approvers.rs`:

```rust
use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::state::{TrustedApprovers, Vault};

pub fn initialize_approvers(ctx: Context<InitializeApprovers>) -> Result<()> {
    let approvers = &mut ctx.accounts.approvers;
    approvers.vault = ctx.accounts.vault.key();
    approvers.count = 0;
    Ok(())
}

#[derive(Accounts)]
pub struct InitializeApprovers<'info> {
    #[account(mut, constraint = payer.key() == vault.owner @ VaultError::NotVaultOwner)]
    pub payer: Signer<'info>,

    pub vault: Account<'info, Vault>,

    #[account(
        init,
        payer = payer,
        space = TrustedApprovers::SPACE,
        seeds = [b"approvers", vault.key().as_ref()],
        bump
    )]
    pub approvers: Account<'info, TrustedApprovers>,

    pub system_program: Program<'info, System>,
}
```

- [ ] **Step 4: Write `add_trusted_approver.rs`**

Create `programs/family-vault/src/instructions/add_trusted_approver.rs`:

```rust
use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::state::{TrustedApprovers, Vault, MAX_APPROVERS};

pub fn add_trusted_approver(ctx: Context<AddTrustedApprover>, address: Pubkey) -> Result<()> {
    let approvers = &mut ctx.accounts.approvers;
    require!((approvers.count as usize) < MAX_APPROVERS, VaultError::ApproverListFull);

    for i in 0..(approvers.count as usize) {
        require!(approvers.entries[i] != address, VaultError::ApproverAlreadyExists);
    }

    let idx = approvers.count as usize;
    approvers.entries[idx] = address;
    approvers.count += 1;

    Ok(())
}

#[derive(Accounts)]
pub struct AddTrustedApprover<'info> {
    #[account(constraint = signer.key() == vault.owner @ VaultError::NotVaultOwner)]
    pub signer: Signer<'info>,

    pub vault: Account<'info, Vault>,

    #[account(mut, has_one = vault)]
    pub approvers: Account<'info, TrustedApprovers>,
}
```

- [ ] **Step 5: Wire both into `instructions/mod.rs`**

Append to the existing file:

```rust
pub mod initialize_approvers;
pub use initialize_approvers::*;
pub mod add_trusted_approver;
pub use add_trusted_approver::*;
```

- [ ] **Step 6: Wire both into `lib.rs`**

Add inside the existing `#[program] pub mod family_vault { ... }` block,
after the existing `withdraw` function:

```rust
    pub fn initialize_approvers(ctx: Context<InitializeApprovers>) -> Result<()> {
        instructions::initialize_approvers(ctx)
    }

    pub fn add_trusted_approver(ctx: Context<AddTrustedApprover>, address: Pubkey) -> Result<()> {
        instructions::add_trusted_approver(ctx, address)
    }
```

- [ ] **Step 7: Build**

Run: `cargo build-sbf --manifest-path programs/family-vault/Cargo.toml`
Expected: build succeeds

- [ ] **Step 8: Write the tests**

Create `programs/family-vault/tests/trusted_approvers.rs`:

```rust
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
    let accounts = family_vault::accounts::InitializeApprovers { payer: owner.pubkey(), vault, approvers };
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
```

- [ ] **Step 9: Run the tests**

Run: `cargo test --test trusted_approvers -- --nocapture`
Expected: all 3 tests PASS

- [ ] **Step 10: Commit**

```bash
git add programs/family-vault/src programs/family-vault/tests
git commit -m "feat: add TrustedApprovers list with initialize_approvers/add_trusted_approver"
```

---

### Task 2: `PendingWithdrawal` + `request_withdrawal` + `approve_withdrawal`

**Files:**
- Modify: `programs/family-vault/src/state.rs`
- Modify: `programs/family-vault/src/errors.rs`
- Create: `programs/family-vault/src/instructions/request_withdrawal.rs`
- Create: `programs/family-vault/src/instructions/approve_withdrawal.rs`
- Modify: `programs/family-vault/src/instructions/mod.rs`
- Modify: `programs/family-vault/src/lib.rs`
- Create: `programs/family-vault/tests/withdrawal_approval.rs`

**Interfaces:**
- Consumes: `state::{Vault, BeneficiaryList, TrustedApprovers}`, `errors::VaultError::{UnknownBeneficiary, BeneficiaryCooldownActive, ZeroAmount}`
- Produces: `state::PendingWithdrawal { vault, amount, to, requested_at, approved_by: Option<Pubkey>, executed, bump }`, `errors::VaultError::{NotPoolVault, RequestAlreadyExecuted, NotTrustedApprover}`, `family_vault::instruction::{RequestWithdrawal, ApproveWithdrawal}`, `family_vault::accounts::{RequestWithdrawal, ApproveWithdrawal}`

- [ ] **Step 1: Add `PendingWithdrawal` to `state.rs`**

Append:

```rust

#[account]
pub struct PendingWithdrawal {
    pub vault: Pubkey,
    pub amount: u64,
    pub to: Pubkey,
    pub requested_at: i64,
    pub approved_by: Option<Pubkey>,
    pub executed: bool,
    pub bump: u8,
}

impl PendingWithdrawal {
    pub const SPACE: usize = 8 // discriminator
        + 32 // vault
        + 8  // amount
        + 32 // to
        + 8  // requested_at
        + (1 + 32) // approved_by: Option<Pubkey>
        + 1  // executed
        + 1; // bump
}
```

- [ ] **Step 2: Add new error variants to `errors.rs`**

Append:

```rust
    #[msg("This instruction is only valid for pool vaults")]
    NotPoolVault,
    #[msg("This withdrawal request has already been executed")]
    RequestAlreadyExecuted,
    #[msg("Signer is not a trusted approver")]
    NotTrustedApprover,
```

- [ ] **Step 3: Write `request_withdrawal.rs`**

Create `programs/family-vault/src/instructions/request_withdrawal.rs`:

```rust
use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

use crate::errors::VaultError;
use crate::state::{BeneficiaryList, PendingWithdrawal, Vault};

pub fn request_withdrawal(ctx: Context<RequestWithdrawal>, amount: u64, _nonce: u64) -> Result<()> {
    require!(amount > 0, VaultError::ZeroAmount);

    let vault = &ctx.accounts.vault;
    require!(vault.is_pool, VaultError::NotPoolVault);

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

    let request = &mut ctx.accounts.pending_withdrawal;
    request.vault = vault.key();
    request.amount = amount;
    request.to = to_key;
    request.requested_at = clock.unix_timestamp;
    request.approved_by = None;
    request.executed = false;
    request.bump = ctx.bumps.pending_withdrawal;

    Ok(())
}

#[derive(Accounts)]
#[instruction(amount: u64, nonce: u64)]
pub struct RequestWithdrawal<'info> {
    #[account(mut, constraint = signer.key() == vault.owner @ VaultError::NotVaultOwner)]
    pub signer: Signer<'info>,

    pub vault: Account<'info, Vault>,

    #[account(has_one = vault)]
    pub beneficiary_list: Account<'info, BeneficiaryList>,

    pub recipient_token_account: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = signer,
        space = PendingWithdrawal::SPACE,
        seeds = [b"request", vault.key().as_ref(), &nonce.to_le_bytes()],
        bump
    )]
    pub pending_withdrawal: Account<'info, PendingWithdrawal>,

    pub system_program: Program<'info, System>,
}
```

- [ ] **Step 4: Write `approve_withdrawal.rs`**

Create `programs/family-vault/src/instructions/approve_withdrawal.rs`:

```rust
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::errors::VaultError;
use crate::state::{PendingWithdrawal, TrustedApprovers, Vault};

pub fn approve_withdrawal(ctx: Context<ApproveWithdrawal>) -> Result<()> {
    require!(!ctx.accounts.pending_withdrawal.executed, VaultError::RequestAlreadyExecuted);

    let signer_key = ctx.accounts.signer.key();
    let approvers = &ctx.accounts.approvers;
    let mut is_approver = false;
    for i in 0..(approvers.count as usize) {
        if approvers.entries[i] == signer_key {
            is_approver = true;
            break;
        }
    }
    require!(is_approver, VaultError::NotTrustedApprover);

    let vault = &ctx.accounts.vault;
    let owner_key = vault.owner;
    let seed_kind: &[u8] = if vault.is_pool { b"pool" } else { b"personal" };
    let bump = vault.bump;
    let vault_seeds: &[&[u8]] = &[b"vault", owner_key.as_ref(), seed_kind, &[bump]];
    let signer_seeds: &[&[&[u8]]] = &[vault_seeds];

    let amount = ctx.accounts.pending_withdrawal.amount;

    let cpi_accounts = Transfer {
        from: ctx.accounts.vault_token_account.to_account_info(),
        to: ctx.accounts.recipient_token_account.to_account_info(),
        authority: vault.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.key(),
        cpi_accounts,
        signer_seeds,
    );
    token::transfer(cpi_ctx, amount)?;

    let request = &mut ctx.accounts.pending_withdrawal;
    request.approved_by = Some(signer_key);
    request.executed = true;

    Ok(())
}

#[derive(Accounts)]
pub struct ApproveWithdrawal<'info> {
    pub signer: Signer<'info>,

    #[account(mut, has_one = vault)]
    pub pending_withdrawal: Account<'info, PendingWithdrawal>,

    #[account(has_one = vault)]
    pub approvers: Account<'info, TrustedApprovers>,

    #[account(mut, address = vault.vault_token_account)]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(mut, constraint = recipient_token_account.owner == pending_withdrawal.to @ VaultError::UnknownBeneficiary)]
    pub recipient_token_account: Account<'info, TokenAccount>,

    pub vault: Account<'info, Vault>,

    pub token_program: Program<'info, Token>,
}
```

- [ ] **Step 5: Wire both into `instructions/mod.rs`**

Append:

```rust
pub mod request_withdrawal;
pub use request_withdrawal::*;
pub mod approve_withdrawal;
pub use approve_withdrawal::*;
```

- [ ] **Step 6: Wire both into `lib.rs`**

Add inside `#[program] pub mod family_vault { ... }`, after
`add_trusted_approver`:

```rust
    pub fn request_withdrawal(ctx: Context<RequestWithdrawal>, amount: u64, nonce: u64) -> Result<()> {
        instructions::request_withdrawal(ctx, amount, nonce)
    }

    pub fn approve_withdrawal(ctx: Context<ApproveWithdrawal>) -> Result<()> {
        instructions::approve_withdrawal(ctx)
    }
```

- [ ] **Step 7: Build**

Run: `cargo build-sbf --manifest-path programs/family-vault/Cargo.toml`
Expected: build succeeds

- [ ] **Step 8: Write the tests**

Create `programs/family-vault/tests/withdrawal_approval.rs`:

```rust
mod common;

use anchor_lang::{InstructionData, ToAccountMetas};
use common::setup;
use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::{instruction::Instruction, system_program};
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
    let (vault, _) = Pubkey::find_program_address(&[b"vault", owner.pubkey().as_ref(), b"pool"], &family_vault::ID);
    let (beneficiary_list, _) = Pubkey::find_program_address(&[b"beneficiaries", vault.as_ref()], &family_vault::ID);
    let vault_token_account = spl_associated_token_account::get_associated_token_address(&vault, &ctx.mint);

    let init_accounts = family_vault::accounts::InitializeVault {
        payer: ctx.payer.pubkey(), owner: owner.pubkey(), vault, beneficiary_list,
        mint: ctx.mint, vault_token_account, token_program: spl_token::id(),
        associated_token_program: spl_associated_token_account::id(), system_program: system_program::id(),
    };
    let init_data = family_vault::instruction::InitializeVault { is_pool: true, daily_limit: DAILY_LIMIT };
    let ix = Instruction { program_id: family_vault::ID, accounts: init_accounts.to_account_metas(None), data: init_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, owner], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx).unwrap();

    // Fund the vault.
    let mint_ix = spl_token::instruction::mint_to(
        &spl_token::id(), &ctx.mint, &vault_token_account, &ctx.mint_authority.pubkey(), &[], 200_000_000,
    ).unwrap();
    let tx = Transaction::new_signed_with_payer(&[mint_ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &ctx.mint_authority], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx).unwrap();

    // Add beneficiary and clear its cooldown.
    let add_ben_accounts = family_vault::accounts::AddBeneficiary { signer: owner.pubkey(), vault, beneficiary_list };
    let add_ben_data = family_vault::instruction::AddBeneficiary { name: "Beneficiary".to_string(), address: beneficiary };
    let ix = Instruction { program_id: family_vault::ID, accounts: add_ben_accounts.to_account_metas(None), data: add_ben_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, owner], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx).unwrap();
    let mut clock = ctx.svm.get_sysvar::<anchor_lang::solana_program::clock::Clock>();
    clock.unix_timestamp += 172_801;
    ctx.svm.set_sysvar(&clock);

    // Initialize approvers and add the approver.
    let (approvers, _) = Pubkey::find_program_address(&[b"approvers", vault.as_ref()], &family_vault::ID);
    let init_app_accounts = family_vault::accounts::InitializeApprovers { payer: owner.pubkey(), vault, approvers };
    let ix = Instruction { program_id: family_vault::ID, accounts: init_app_accounts.to_account_metas(None), data: family_vault::instruction::InitializeApprovers {}.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, owner], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx).unwrap();

    let add_app_accounts = family_vault::accounts::AddTrustedApprover { signer: owner.pubkey(), vault, approvers };
    let add_app_data = family_vault::instruction::AddTrustedApprover { address: approver.pubkey() };
    let ix = Instruction { program_id: family_vault::ID, accounts: add_app_accounts.to_account_metas(None), data: add_app_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, owner], ctx.svm.latest_blockhash());
    ctx.svm.send_transaction(tx).unwrap();

    PoolVaultFixture { vault, vault_token_account, beneficiary_list, approvers }
}

fn create_ata(ctx: &mut common::TestCtx, owner: &Pubkey) -> Pubkey {
    let ix = spl_associated_token_account::instruction::create_associated_token_account(
        &ctx.payer.pubkey(), owner, &ctx.mint, &spl_token::id(),
    );
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer], ctx.svm.latest_blockhash());
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
    ctx.svm.airdrop(&beneficiary.pubkey(), 1_000_000_000).unwrap();

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
    let request_data = family_vault::instruction::RequestWithdrawal { amount: 80_000_000, nonce };
    let ix = Instruction { program_id: family_vault::ID, accounts: request_accounts.to_account_metas(None), data: request_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &owner], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_ok(), "request_withdrawal failed: {:?}", result);
    assert_eq!(token_balance(&ctx, &beneficiary_ata), 0, "requesting must not move funds yet");

    // Approver approves — this executes the transfer.
    let approve_accounts = family_vault::accounts::ApproveWithdrawal {
        signer: approver.pubkey(),
        pending_withdrawal,
        approvers: fixture.approvers,
        vault_token_account: fixture.vault_token_account,
        recipient_token_account: beneficiary_ata,
        vault: fixture.vault,
        token_program: spl_token::id(),
    };
    let ix = Instruction { program_id: family_vault::ID, accounts: approve_accounts.to_account_metas(None), data: family_vault::instruction::ApproveWithdrawal {}.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &approver], ctx.svm.latest_blockhash());
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
    ctx.svm.airdrop(&beneficiary.pubkey(), 1_000_000_000).unwrap();
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
    let request_data = family_vault::instruction::RequestWithdrawal { amount: 80_000_000, nonce };
    let ix = Instruction { program_id: family_vault::ID, accounts: request_accounts.to_account_metas(None), data: request_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &owner], ctx.svm.latest_blockhash());
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
    let ix = Instruction { program_id: family_vault::ID, accounts: approve_accounts.to_account_metas(None), data: family_vault::instruction::ApproveWithdrawal {}.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &stranger], ctx.svm.latest_blockhash());
    let result = ctx.svm.send_transaction(tx);
    assert!(result.is_err(), "expected non-approver approve_withdrawal to be rejected");
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
    ctx.svm.airdrop(&beneficiary.pubkey(), 1_000_000_000).unwrap();

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
    let request_data = family_vault::instruction::RequestWithdrawal { amount: 60_000_000, nonce };
    let ix = Instruction { program_id: family_vault::ID, accounts: request_accounts.to_account_metas(None), data: request_data.data() };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&ctx.payer.pubkey()), &[&ctx.payer, &owner], ctx.svm.latest_blockhash());
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
    let ix1 = Instruction { program_id: family_vault::ID, accounts: approve_accounts.to_account_metas(None), data: family_vault::instruction::ApproveWithdrawal {}.data() };
    let tx1 = Transaction::new_signed_with_payer(&[ix1], Some(&ctx.payer.pubkey()), &[&ctx.payer, &approver], ctx.svm.latest_blockhash());
    let result1 = ctx.svm.send_transaction(tx1);
    assert!(result1.is_ok(), "first approve_withdrawal failed: {:?}", result1);

    let ix2 = Instruction { program_id: family_vault::ID, accounts: approve_accounts.to_account_metas(None), data: family_vault::instruction::ApproveWithdrawal {}.data() };
    let tx2 = Transaction::new_signed_with_payer(&[ix2], Some(&ctx.payer.pubkey()), &[&ctx.payer, &approver], ctx.svm.latest_blockhash());
    let result2 = ctx.svm.send_transaction(tx2);
    assert!(result2.is_err(), "expected second approve_withdrawal on the same request to be rejected");
    assert_eq!(token_balance(&ctx, &beneficiary_ata), 60_000_000, "balance must reflect exactly one execution");
}
```

- [ ] **Step 9: Run the tests**

Run: `cargo test --test withdrawal_approval -- --nocapture`
Expected: all 3 tests PASS

- [ ] **Step 10: Run the full suite to confirm no regressions**

Run: `cargo test -- --test-threads=1`
Expected: every existing test (22 from `family-vault-core`) plus all new
tests from this plan still pass

- [ ] **Step 11: Commit**

```bash
git add programs/family-vault/src programs/family-vault/tests
git commit -m "feat: add request_withdrawal/approve_withdrawal for over-limit pool withdrawals"
```

---

### Task 3: `PendingRotation` + owner-rotation recovery

**Files:**
- Modify: `programs/family-vault/src/state.rs`
- Modify: `programs/family-vault/src/errors.rs`
- Create: `programs/family-vault/src/instructions/request_owner_rotation.rs`
- Create: `programs/family-vault/src/instructions/cancel_owner_rotation.rs`
- Create: `programs/family-vault/src/instructions/execute_owner_rotation.rs`
- Modify: `programs/family-vault/src/instructions/mod.rs`
- Modify: `programs/family-vault/src/lib.rs`
- Create: `programs/family-vault/tests/owner_rotation.rs`

**Interfaces:**
- Consumes: `state::{Vault, TrustedApprovers}`
- Produces: `state::{PendingRotation, ROTATION_TIMELOCK_SECS}`, `errors::VaultError::{RotationAlreadyCancelled, NotAuthorizedToCancel, RotationTimelockNotElapsed}`, `family_vault::instruction::{RequestOwnerRotation, CancelOwnerRotation, ExecuteOwnerRotation}`

- [ ] **Step 1: Add `PendingRotation` + timelock constant to `state.rs`**

Append:

```rust

pub const ROTATION_TIMELOCK_SECS: i64 = 259_200; // 72h

#[account]
pub struct PendingRotation {
    pub vault: Pubkey,
    pub new_owner: Pubkey,
    pub requested_by: Pubkey,
    pub requested_at: i64,
    pub executes_at: i64,
    pub cancelled: bool,
    pub bump: u8,
}

impl PendingRotation {
    pub const SPACE: usize = 8 // discriminator
        + 32 // vault
        + 32 // new_owner
        + 32 // requested_by
        + 8  // requested_at
        + 8  // executes_at
        + 1  // cancelled
        + 1; // bump
}
```

- [ ] **Step 2: Add new error variants to `errors.rs`**

Append:

```rust
    #[msg("This rotation request has already been cancelled")]
    RotationAlreadyCancelled,
    #[msg("Only the vault owner or a trusted approver may cancel a rotation")]
    NotAuthorizedToCancel,
    #[msg("The rotation timelock has not elapsed yet")]
    RotationTimelockNotElapsed,
```

- [ ] **Step 3: Write `request_owner_rotation.rs`**

Create `programs/family-vault/src/instructions/request_owner_rotation.rs`:

```rust
use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::state::{PendingRotation, TrustedApprovers, Vault, ROTATION_TIMELOCK_SECS};

pub fn request_owner_rotation(ctx: Context<RequestOwnerRotation>, new_owner: Pubkey, _nonce: u64) -> Result<()> {
    let signer_key = ctx.accounts.signer.key();
    let approvers = &ctx.accounts.approvers;
    let mut is_approver = false;
    for i in 0..(approvers.count as usize) {
        if approvers.entries[i] == signer_key {
            is_approver = true;
            break;
        }
    }
    require!(is_approver, VaultError::NotTrustedApprover);

    let now = Clock::get()?.unix_timestamp;
    let executes_at = now
        .checked_add(ROTATION_TIMELOCK_SECS)
        .ok_or(VaultError::AmountOverflow)?;

    let rotation = &mut ctx.accounts.rotation;
    rotation.vault = ctx.accounts.vault.key();
    rotation.new_owner = new_owner;
    rotation.requested_by = signer_key;
    rotation.requested_at = now;
    rotation.executes_at = executes_at;
    rotation.cancelled = false;
    rotation.bump = ctx.bumps.rotation;

    Ok(())
}

#[derive(Accounts)]
#[instruction(new_owner: Pubkey, nonce: u64)]
pub struct RequestOwnerRotation<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    pub vault: Account<'info, Vault>,

    #[account(has_one = vault)]
    pub approvers: Account<'info, TrustedApprovers>,

    #[account(
        init,
        payer = signer,
        space = PendingRotation::SPACE,
        seeds = [b"rotation", vault.key().as_ref(), &nonce.to_le_bytes()],
        bump
    )]
    pub rotation: Account<'info, PendingRotation>,

    pub system_program: Program<'info, System>,
}
```

- [ ] **Step 4: Write `cancel_owner_rotation.rs`**

Create `programs/family-vault/src/instructions/cancel_owner_rotation.rs`:

```rust
use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::state::{PendingRotation, TrustedApprovers, Vault};

pub fn cancel_owner_rotation(ctx: Context<CancelOwnerRotation>) -> Result<()> {
    require!(!ctx.accounts.rotation.cancelled, VaultError::RotationAlreadyCancelled);

    let signer_key = ctx.accounts.signer.key();
    let vault = &ctx.accounts.vault;
    let is_owner = signer_key == vault.owner;

    let approvers = &ctx.accounts.approvers;
    let mut is_approver = false;
    for i in 0..(approvers.count as usize) {
        if approvers.entries[i] == signer_key {
            is_approver = true;
            break;
        }
    }

    require!(is_owner || is_approver, VaultError::NotAuthorizedToCancel);

    ctx.accounts.rotation.cancelled = true;
    Ok(())
}

#[derive(Accounts)]
pub struct CancelOwnerRotation<'info> {
    pub signer: Signer<'info>,

    pub vault: Account<'info, Vault>,

    #[account(has_one = vault)]
    pub approvers: Account<'info, TrustedApprovers>,

    #[account(mut, has_one = vault)]
    pub rotation: Account<'info, PendingRotation>,
}
```

- [ ] **Step 5: Write `execute_owner_rotation.rs`**

Create `programs/family-vault/src/instructions/execute_owner_rotation.rs`:

```rust
use anchor_lang::prelude::*;

use crate::errors::VaultError;
use crate::state::{PendingRotation, Vault};

pub fn execute_owner_rotation(ctx: Context<ExecuteOwnerRotation>) -> Result<()> {
    let rotation = &ctx.accounts.rotation;
    require!(!rotation.cancelled, VaultError::RotationAlreadyCancelled);

    let now = Clock::get()?.unix_timestamp;
    require!(now >= rotation.executes_at, VaultError::RotationTimelockNotElapsed);

    let new_owner = rotation.new_owner;
    ctx.accounts.vault.owner = new_owner;

    Ok(())
}

#[derive(Accounts)]
pub struct ExecuteOwnerRotation<'info> {
    // Permissionless: anyone may trigger execution once the timelock has
    // elapsed. `Signer` only proves this transaction was submitted with a
    // valid signature — it does not gate who that signer may be.
    pub signer: Signer<'info>,

    #[account(mut)]
    pub vault: Account<'info, Vault>,

    #[account(has_one = vault)]
    pub rotation: Account<'info, PendingRotation>,
}
```

- [ ] **Step 6: Wire all three into `instructions/mod.rs`**

Append:

```rust
pub mod request_owner_rotation;
pub use request_owner_rotation::*;
pub mod cancel_owner_rotation;
pub use cancel_owner_rotation::*;
pub mod execute_owner_rotation;
pub use execute_owner_rotation::*;
```

- [ ] **Step 7: Wire all three into `lib.rs`**

Add inside `#[program] pub mod family_vault { ... }`, after
`approve_withdrawal`:

```rust
    pub fn request_owner_rotation(ctx: Context<RequestOwnerRotation>, new_owner: Pubkey, nonce: u64) -> Result<()> {
        instructions::request_owner_rotation(ctx, new_owner, nonce)
    }

    pub fn cancel_owner_rotation(ctx: Context<CancelOwnerRotation>) -> Result<()> {
        instructions::cancel_owner_rotation(ctx)
    }

    pub fn execute_owner_rotation(ctx: Context<ExecuteOwnerRotation>) -> Result<()> {
        instructions::execute_owner_rotation(ctx)
    }
```

- [ ] **Step 8: Build**

Run: `cargo build-sbf --manifest-path programs/family-vault/Cargo.toml`
Expected: build succeeds

- [ ] **Step 9: Write the tests**

Create `programs/family-vault/tests/owner_rotation.rs`:

```rust
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
    let init_app_accounts = family_vault::accounts::InitializeApprovers { payer: owner.pubkey(), vault, approvers };
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

    // Warp past the 72h timelock.
    let mut clock = ctx.svm.get_sysvar::<anchor_lang::solana_program::clock::Clock>();
    clock.unix_timestamp += 259_201;
    ctx.svm.set_sysvar(&clock);

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
```

- [ ] **Step 10: Run the tests**

Run: `cargo test --test owner_rotation -- --nocapture`
Expected: all 3 tests PASS

- [ ] **Step 11: Run the full suite one final time**

Run: `cargo test -- --test-threads=1`
Expected: every test in the program (from both `family-vault-core` and
this plan) passes

- [ ] **Step 12: Commit**

```bash
git add programs/family-vault/src programs/family-vault/tests
git commit -m "feat: add owner-rotation recovery (request/cancel/execute) with 72h timelock"
```

---

## Out of scope for this plan (unchanged from the design spec)

- Kiosk GUI / signer integration (separate plan).
- Alpine + cage OS image (separate plan).
- Devnet deployment (manual step once both on-chain plans are done).
- Multi-family / multi-admin scenarios (explicitly out of scope per spec).
