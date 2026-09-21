# Kiosk GUI Blockchain Wiring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the mock data and `println!` stub in `kiosk-gui/src/main.rs` with real Solana devnet calls: read `Vault`/`BeneficiaryList` accounts on startup, sign and send a real `withdraw` transaction when the user confirms a send.

**Architecture:** New module `kiosk-gui/src/chain.rs` holds all blockchain logic (config, account decoding, instruction building, RPC calls). Blocking `solana_client::rpc_client::RpcClient` calls run on `std::thread::spawn`, never on the UI thread; results marshal back via `slint::invoke_from_event_loop`. `main.rs` is only wired to call into `chain.rs` and update Slint properties — it does not know about RPC/Pubkey types itself beyond what's needed to pass through.

**Tech Stack:** Rust, Slint 1.18.0, `solana-client` 4.2.2 (blocking `RpcClient`), granular `solana-*` client crates (see Task 1 — versions were empirically verified against this workspace's toolchain, do not substitute different versions), `family-vault` crate itself as a dependency (with `cpi` feature) for its Anchor-generated `instruction`/`accounts` client bindings, `toml` + `serde` for config.

**Spec:** `docs/superpowers/specs/2026-09-21-kiosk-blockchain-wiring-design.md` (also read the two specs it inherits from: `docs/superpowers/specs/2026-09-12-family-vault-design.md`, `docs/superpowers/specs/2026-09-17-kiosk-gui-design.md`)

## Global Constraints

- Amount decimals are **fixed at 6** (`TOKEN_DECIMALS: u32 = 6`) — do not read decimals from the Mint account (spec: "Decimals" section — deliberate simplification for MVP).
- The UI thread must never block on network I/O — all RPC calls run on a background `std::thread::spawn`, results delivered via `slint::invoke_from_event_loop` (spec: "Архитектура").
- This plan implements **`withdraw` only** — never `request_withdrawal`/over-limit flow. That belongs to a separate family web app (spec: "Вне скоупа этого plan'а").
- Error text shown in the UI must stay in the two fixed forms already defined by the GUI shell spec: "Нет связи. Попробуй ещё раз позже" (network failure) and "Не получилось. Попробуй ещё раз или позови [родственника]" (any program/transaction failure). Raw error text goes to `eprintln!`/log only, never to a Slint property (spec: "Обработка ошибок" in the parent kiosk-gui-design spec).
- `Beneficiary.address` is a wallet pubkey, not a token account — the recipient's Associated Token Account (ATA) must be derived via `spl_associated_token_account::get_associated_token_address`.

---

### Task 1: Dependencies + config loading

**Files:**
- Modify: `kiosk-gui/Cargo.toml`
- Create: `kiosk-gui/src/chain.rs`
- Modify: `kiosk-gui/src/main.rs:1` (add `mod chain;`)
- Test: inline `#[test]` module inside `kiosk-gui/src/chain.rs`

**Interfaces:**
- Produces: `chain::ChainConfig` struct, `chain::ConfigError` enum, `chain::load_config(path: &std::path::Path) -> Result<ChainConfig, ConfigError>`, `chain::TOKEN_DECIMALS: u32 = 6` constant.

These exact dependency versions were verified to compile together against
this workspace's installed `rustc 1.96.0` and the existing `family-vault`
crate (which itself already pins `solana-message`/`solana-transaction`/
`solana-signer`/`solana-keypair` in its dev-dependencies at compatible
versions) — **do not `cargo add` with different/latest versions**, the
Solana crate ecosystem has multiple incompatible major versions in flight
simultaneously and picking a different one will very likely break the
build with `Pubkey`/`Instruction` type-identity errors that are hard to
debug from the error message alone.

- [ ] **Step 1: Add dependencies to `kiosk-gui/Cargo.toml`**

Add to the `[dependencies]` section (alongside the existing `slint = "1.18.0"`):

```toml
anchor-lang = "1.2.0"
family-vault = { version = "0.1.0", path = "../programs/family-vault", features = ["cpi"] }
serde = { version = "1.0.229", features = ["derive"] }
solana-client = "=4.2.2"
solana-hash = "3.0.0"
solana-instruction = "3.0.0"
solana-keypair = "3.0.1"
solana-message = { version = "4.1.0", features = ["wincode"] }
solana-pubkey = "=4.2.1"
solana-signature = "=3.4.1"
solana-signer = "3.0.0"
solana-transaction = { version = "4.1.6", features = ["wincode"] }
spl-associated-token-account = "8.0.0"
spl-token = "9.0.0"
toml = "1.1.6"
```

Run `cargo check` from the repo root. Expected: succeeds (warnings about
`unexpected cfg condition value: anchor-debug` coming from the
`family-vault` crate's own macros are pre-existing and harmless — they are
not from this change).

- [ ] **Step 2: Add `mod chain;` to `main.rs`**

At the top of `kiosk-gui/src/main.rs`, right after `slint::include_modules!();`:

```rust
slint::include_modules!();

mod chain;
```

- [ ] **Step 3: Create `kiosk-gui/src/chain.rs` with config types**

```rust
use std::fmt;
use std::path::Path;

use serde::Deserialize;

/// SPL token decimals for the demo token used on devnet. Fixed, not read
/// from the Mint account — see the design spec's "Decimals" section.
pub const TOKEN_DECIMALS: u32 = 6;

#[derive(Debug, Deserialize)]
struct RawConfig {
    rpc_url: String,
    keypair_path: String,
    program_id: String,
    mint: String,
    personal_vault: String,
    personal_beneficiary_list: String,
    pool_vault: String,
    pool_beneficiary_list: String,
}

pub struct ChainConfig {
    pub rpc_url: String,
    pub keypair_path: String,
    pub program_id: anchor_lang::prelude::Pubkey,
    pub mint: solana_pubkey::Pubkey,
    pub personal_vault: solana_pubkey::Pubkey,
    pub personal_beneficiary_list: solana_pubkey::Pubkey,
    pub pool_vault: solana_pubkey::Pubkey,
    pub pool_beneficiary_list: solana_pubkey::Pubkey,
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(toml::de::Error),
    BadPubkey { field: &'static str, value: String },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "could not read config file: {e}"),
            ConfigError::Parse(e) => write!(f, "could not parse config TOML: {e}"),
            ConfigError::BadPubkey { field, value } => {
                write!(f, "config field '{field}' is not a valid pubkey: '{value}'")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

fn parse_client_pubkey(field: &'static str, value: &str) -> Result<solana_pubkey::Pubkey, ConfigError> {
    value.parse().map_err(|_| ConfigError::BadPubkey {
        field,
        value: value.to_string(),
    })
}

pub fn load_config(path: &Path) -> Result<ChainConfig, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(ConfigError::Io)?;
    let raw: RawConfig = toml::from_str(&text).map_err(ConfigError::Parse)?;

    let program_id: anchor_lang::prelude::Pubkey =
        raw.program_id.parse().map_err(|_| ConfigError::BadPubkey {
            field: "program_id",
            value: raw.program_id.clone(),
        })?;

    Ok(ChainConfig {
        rpc_url: raw.rpc_url,
        keypair_path: raw.keypair_path,
        program_id,
        mint: parse_client_pubkey("mint", &raw.mint)?,
        personal_vault: parse_client_pubkey("personal_vault", &raw.personal_vault)?,
        personal_beneficiary_list: parse_client_pubkey(
            "personal_beneficiary_list",
            &raw.personal_beneficiary_list,
        )?,
        pool_vault: parse_client_pubkey("pool_vault", &raw.pool_vault)?,
        pool_beneficiary_list: parse_client_pubkey(
            "pool_beneficiary_list",
            &raw.pool_beneficiary_list,
        )?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_temp_config(contents: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("kiosk-test-config-{}.toml", std::process::id()));
        std::fs::write(&path, contents).unwrap();
        path
    }

    const VALID_TOML: &str = r#"
rpc_url = "https://api.devnet.solana.com"
keypair_path = "/etc/family-vault/owner-keypair.json"
program_id = "FXFxTexyhpdjmb6s2kvjVeY2aJFD9jmW8PsfSUY6LF7m"
mint = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"
personal_vault = "11111111111111111111111111111111111111111"
personal_beneficiary_list = "11111111111111111111111111111111111111111"
pool_vault = "11111111111111111111111111111111111111111"
pool_beneficiary_list = "11111111111111111111111111111111111111111"
"#;

    #[test]
    fn loads_a_valid_config() {
        let path = write_temp_config(VALID_TOML);
        let cfg = load_config(&path).unwrap();
        assert_eq!(cfg.rpc_url, "https://api.devnet.solana.com");
        assert_eq!(cfg.keypair_path, "/etc/family-vault/owner-keypair.json");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn rejects_missing_file() {
        let path = std::env::temp_dir().join("kiosk-test-config-does-not-exist.toml");
        let err = load_config(&path).unwrap_err();
        assert!(matches!(err, ConfigError::Io(_)));
    }

    #[test]
    fn rejects_bad_pubkey() {
        let bad = VALID_TOML.replace(
            "mint = \"Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB\"",
            "mint = \"not-a-pubkey\"",
        );
        let path = write_temp_config(&bad);
        let err = load_config(&path).unwrap_err();
        assert!(matches!(err, ConfigError::BadPubkey { field: "mint", .. }));
        std::fs::remove_file(&path).unwrap();
    }
}
```

Note: `"11111111111111111111111111111111111111111"` (the System Program
address, 43 chars but valid base58 — used here purely as a syntactically
valid placeholder pubkey for the test) parses fine as any `Pubkey` type;
the test doesn't care that it's not really a vault address.

- [ ] **Step 4: Run the tests**

Run: `cargo test --package kiosk-gui chain::tests -- --nocapture`
Expected: 3 tests pass (`loads_a_valid_config`, `rejects_missing_file`, `rejects_bad_pubkey`).

- [ ] **Step 5: Commit**

```bash
git add kiosk-gui/Cargo.toml kiosk-gui/Cargo.lock kiosk-gui/src/main.rs kiosk-gui/src/chain.rs
git commit -m "feat(kiosk-gui): add blockchain client deps and config loading"
```

---

### Task 2: Amount formatting and beneficiary name decoding

**Files:**
- Modify: `kiosk-gui/src/chain.rs`

**Interfaces:**
- Consumes: `TOKEN_DECIMALS` (Task 1).
- Produces: `chain::format_amount(whole_units: u64) -> String`,
  `chain::parse_amount_to_base_units(whole_units: u64) -> u64`,
  `chain::decode_beneficiary_name(name: &[u8; 32], name_len: u8) -> String`.

These are pure functions — no network, no RPC types — used by both the
account-fetching code (Task 3) and the withdraw-submission code (Task 4).

- [ ] **Step 1: Write the failing tests**

Add to `kiosk-gui/src/chain.rs`, inside the existing `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn formats_whole_units_as_usdc_string() {
        assert_eq!(format_amount(10), "10 USDC");
        assert_eq!(format_amount(0), "0 USDC");
        assert_eq!(format_amount(120), "120 USDC");
    }

    #[test]
    fn converts_whole_units_to_base_units_with_six_decimals() {
        assert_eq!(parse_amount_to_base_units(1), 1_000_000);
        assert_eq!(parse_amount_to_base_units(10), 10_000_000);
        assert_eq!(parse_amount_to_base_units(0), 0);
    }

    #[test]
    fn decodes_a_beneficiary_name_from_fixed_size_buffer() {
        let mut name = [0u8; 32];
        let src = "Внук Саша".as_bytes();
        name[..src.len()].copy_from_slice(src);
        assert_eq!(
            decode_beneficiary_name(&name, src.len() as u8),
            "Внук Саша"
        );
    }

    #[test]
    fn decodes_an_empty_beneficiary_name() {
        let name = [0u8; 32];
        assert_eq!(decode_beneficiary_name(&name, 0), "");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --package kiosk-gui chain::tests -- --nocapture`
Expected: FAIL with "cannot find function `format_amount`" (and similarly for the other two).

- [ ] **Step 3: Implement the functions**

Add to `kiosk-gui/src/chain.rs`, above the `#[cfg(test)]` block:

```rust
pub fn format_amount(whole_units: u64) -> String {
    format!("{whole_units} USDC")
}

pub fn parse_amount_to_base_units(whole_units: u64) -> u64 {
    whole_units * 10u64.pow(TOKEN_DECIMALS)
}

pub fn decode_beneficiary_name(name: &[u8; 32], name_len: u8) -> String {
    let len = name_len as usize;
    String::from_utf8_lossy(&name[..len.min(32)]).into_owned()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --package kiosk-gui chain::tests -- --nocapture`
Expected: all 7 tests pass (3 from Task 1 + 4 new).

- [ ] **Step 5: Commit**

```bash
git add kiosk-gui/src/chain.rs
git commit -m "feat(kiosk-gui): pure amount/name formatting helpers"
```

---

### Task 3: Vault snapshot decoding and fetching

**Files:**
- Modify: `kiosk-gui/src/chain.rs`

**Interfaces:**
- Consumes: `ChainConfig` (Task 1), `format_amount`, `decode_beneficiary_name` (Task 2).
- Produces: `chain::VaultSnapshot { balance_text: String, beneficiaries: Vec<(u8, String)> }`,
  `chain::ChainError` enum (`NoConnection`, `ProgramRejected(String)`),
  `chain::fetch_vault_snapshot(cfg: &ChainConfig, is_pool: bool) -> Result<VaultSnapshot, ChainError>`.

`beneficiaries` is `(local_id, name)` where `local_id` is the beneficiary's
1-indexed position in the on-chain `BeneficiaryList` — this is the same
`id: i32` the Slint `BeneficiaryItem` model already uses (see
`kiosk-gui/src/main.rs`'s current mock: `BeneficiaryItem { name: ..., id: 1 }`).

The decode logic is split into a pure function (`decode_snapshot`, testable
offline by hand-constructing account bytes) from the RPC-calling wrapper
(`fetch_vault_snapshot`, which cannot be unit-tested without a live
devnet — covered by the manual test procedure in Task 5).

- [ ] **Step 1: Write the failing test for pure decoding**

Add to `kiosk-gui/src/chain.rs`'s test module. This test builds real
on-chain-shaped bytes using the `family_vault` crate's own account types
and `AccountSerialize` (the same trait Anchor uses on-chain to write
account data, including the 8-byte discriminator prefix), then feeds
those bytes through `decode_snapshot` exactly as if they'd come back from
an RPC call:

```rust
    #[test]
    fn decodes_a_vault_and_beneficiary_list_into_a_snapshot() {
        use anchor_lang::AccountSerialize;
        use family_vault::state::{
            Beneficiary, BeneficiaryList, Vault, BENEFICIARY_NAME_LEN, MAX_BENEFICIARIES,
        };

        let vault = Vault {
            owner: Default::default(),
            vault_token_account: Default::default(),
            daily_limit: u64::MAX,
            spent_today: 0,
            last_reset_day: 0,
            is_pool: false,
            bump: 255,
            seed_owner: Default::default(),
        };
        let mut vault_bytes = Vec::new();
        vault.try_serialize(&mut vault_bytes).unwrap();

        let mut name = [0u8; BENEFICIARY_NAME_LEN];
        let src = "Внук Саша".as_bytes();
        name[..src.len()].copy_from_slice(src);
        let mut entries = [Beneficiary::default(); MAX_BENEFICIARIES];
        entries[0] = Beneficiary {
            address: Default::default(),
            name,
            name_len: src.len() as u8,
            added_at: 0,
            active_after: 0,
        };
        let list = BeneficiaryList {
            vault: Default::default(),
            count: 1,
            entries,
        };
        let mut list_bytes = Vec::new();
        list.try_serialize(&mut list_bytes).unwrap();

        let snapshot = decode_snapshot(&vault_bytes, &list_bytes, 3).unwrap();
        assert_eq!(snapshot.balance_text, "3 USDC");
        assert_eq!(snapshot.beneficiaries, vec![(1u8, "Внук Саша".to_string())]);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package kiosk-gui chain::tests::decodes_a_vault_and_beneficiary_list_into_a_snapshot -- --nocapture`
Expected: FAIL with "cannot find function `decode_snapshot`" (and `VaultSnapshot` unresolved).

- [ ] **Step 3: Implement `VaultSnapshot`, `ChainError`, and `decode_snapshot`**

Add to `kiosk-gui/src/chain.rs`, above the test module:

```rust
#[derive(Debug, PartialEq, Eq)]
pub struct VaultSnapshot {
    pub balance_text: String,
    pub beneficiaries: Vec<(u8, String)>,
}

#[derive(Debug)]
pub enum ChainError {
    /// RPC endpoint unreachable — maps to "Нет связи" on Экран 4.
    NoConnection,
    /// RPC responded but the program/transaction rejected the request, or
    /// any other non-network failure — maps to "Не получилось" on Экран 4.
    ProgramRejected(String),
}

impl fmt::Display for ChainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChainError::NoConnection => write!(f, "no connection to RPC endpoint"),
            ChainError::ProgramRejected(msg) => write!(f, "program rejected: {msg}"),
        }
    }
}

impl std::error::Error for ChainError {}

/// Pure decode step, separated from `fetch_vault_snapshot` so it's testable
/// without a live RPC endpoint. `whole_unit_balance` is the vault's token
/// account balance already converted to whole units by the caller (see
/// `fetch_vault_snapshot`, which reads it via a separate RPC call).
fn decode_snapshot(
    vault_bytes: &[u8],
    beneficiary_list_bytes: &[u8],
    whole_unit_balance: u64,
) -> Result<VaultSnapshot, ChainError> {
    use anchor_lang::AccountDeserialize;
    use family_vault::state::BeneficiaryList;

    let list = BeneficiaryList::try_deserialize(&mut &beneficiary_list_bytes[..])
        .map_err(|e| ChainError::ProgramRejected(e.to_string()))?;

    let beneficiaries = (0..list.count as usize)
        .map(|i| {
            let b = &list.entries[i];
            ((i + 1) as u8, decode_beneficiary_name(&b.name, b.name_len))
        })
        .collect();

    // `vault_bytes` isn't read for balance (the token balance comes from a
    // separate token-account RPC call in `fetch_vault_snapshot`), but it's
    // still deserialized here so a corrupt/wrong-shaped Vault account fails
    // loudly instead of silently producing a snapshot from a bogus account.
    use family_vault::state::Vault;
    Vault::try_deserialize(&mut &vault_bytes[..])
        .map_err(|e| ChainError::ProgramRejected(e.to_string()))?;

    Ok(VaultSnapshot {
        balance_text: format_amount(whole_unit_balance),
        beneficiaries,
    })
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package kiosk-gui chain::tests::decodes_a_vault_and_beneficiary_list_into_a_snapshot -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Implement `fetch_vault_snapshot`**

Add to `kiosk-gui/src/chain.rs`, above `decode_snapshot`:

```rust
pub fn fetch_vault_snapshot(cfg: &ChainConfig, is_pool: bool) -> Result<VaultSnapshot, ChainError> {
    let rpc = solana_client::rpc_client::RpcClient::new(cfg.rpc_url.clone());

    let (vault_pubkey, list_pubkey) = if is_pool {
        (cfg.pool_vault, cfg.pool_beneficiary_list)
    } else {
        (cfg.personal_vault, cfg.personal_beneficiary_list)
    };

    let vault_bytes = rpc
        .get_account_data(&vault_pubkey)
        .map_err(map_client_error)?;
    let list_bytes = rpc
        .get_account_data(&list_pubkey)
        .map_err(map_client_error)?;

    // Read the vault's token account balance directly (already in whole
    // units via `ui_amount`, so no manual decimals math needed here).
    use anchor_lang::AccountDeserialize;
    let vault = family_vault::state::Vault::try_deserialize(&mut &vault_bytes[..])
        .map_err(|e| ChainError::ProgramRejected(e.to_string()))?;
    let vault_token_pubkey =
        solana_pubkey::Pubkey::new_from_array(vault.vault_token_account.to_bytes());
    let balance = rpc
        .get_token_account_balance(&vault_token_pubkey)
        .map_err(map_client_error)?;
    let whole_unit_balance: u64 = balance
        .ui_amount
        .map(|f| f.round() as u64)
        .unwrap_or(0);

    decode_snapshot(&vault_bytes, &list_bytes, whole_unit_balance)
}

fn map_client_error(err: solana_client::client_error::ClientError) -> ChainError {
    match err.kind() {
        solana_client::client_error::ClientErrorKind::Io(_)
        | solana_client::client_error::ClientErrorKind::Reqwest(_) => ChainError::NoConnection,
        other => ChainError::ProgramRejected(other.to_string()),
    }
}
```

- [ ] **Step 6: Run the full test suite to make sure nothing broke**

Run: `cargo test --package kiosk-gui chain:: -- --nocapture`
Expected: all tests still pass (`fetch_vault_snapshot` itself has no unit
test — it needs a live RPC endpoint, covered by the manual devnet
procedure in Task 5's testing section).

- [ ] **Step 7: Commit**

```bash
git add kiosk-gui/src/chain.rs
git commit -m "feat(kiosk-gui): decode and fetch Vault/BeneficiaryList snapshots"
```

---

### Task 4: Withdraw submission

**Files:**
- Modify: `kiosk-gui/src/chain.rs`

**Interfaces:**
- Consumes: `ChainConfig`, `ChainError`, `map_client_error`, `parse_amount_to_base_units` (Tasks 1-3).
- Produces: `chain::submit_withdraw(cfg: &ChainConfig, is_pool: bool, beneficiary_address: solana_pubkey::Pubkey, amount_whole_units: u64) -> Result<(), ChainError>`.

`beneficiary_address` is the recipient's **wallet** pubkey (i.e.
`Beneficiary.address` from the on-chain list) — `submit_withdraw` derives
the ATA internally, callers never derive it themselves. `main.rs` (Task 5)
is responsible for keeping the last-fetched beneficiary list around so it
can map a clicked `id: i32` back to a wallet pubkey before calling this
function — that mapping isn't `chain.rs`'s job, since `chain.rs` doesn't
own any UI-facing state.

- [ ] **Step 1: Write the failing test for ATA derivation determinism**

Add to `kiosk-gui/src/chain.rs`'s test module:

```rust
    #[test]
    fn derives_the_same_ata_for_the_same_wallet_and_mint() {
        let wallet = solana_pubkey::Pubkey::new_unique();
        let mint = solana_pubkey::Pubkey::new_unique();
        let ata1 = spl_associated_token_account::get_associated_token_address(&wallet, &mint);
        let ata2 = spl_associated_token_account::get_associated_token_address(&wallet, &mint);
        assert_eq!(ata1, ata2);
        assert_ne!(ata1, wallet);
    }

    #[test]
    fn maps_io_errors_to_no_connection() {
        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
        let client_err: solana_client::client_error::ClientError = io_err.into();
        assert!(matches!(map_client_error(client_err), ChainError::NoConnection));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --package kiosk-gui chain::tests -- --nocapture`
Expected: `derives_the_same_ata_for_the_same_wallet_and_mint` should
already pass (it only exercises `spl_associated_token_account`, already a
dependency) — confirm that. `maps_io_errors_to_no_connection` should also
already pass since `map_client_error` exists from Task 3 — this step is
here to confirm both, not to introduce new failures. If both already
pass, skip straight to Step 3 (there's no new function to implement for
these two — they're regression tests for existing Task 3 code, added here
because `submit_withdraw` is about to depend on both behaviors being
correct).

- [ ] **Step 3: Implement `submit_withdraw`**

Add to `kiosk-gui/src/chain.rs`:

```rust
pub fn submit_withdraw(
    cfg: &ChainConfig,
    is_pool: bool,
    beneficiary_address: solana_pubkey::Pubkey,
    amount_whole_units: u64,
) -> Result<(), ChainError> {
    use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};

    let rpc = solana_client::rpc_client::RpcClient::new(cfg.rpc_url.clone());

    let owner_keypair = solana_keypair::read_keypair_file(&cfg.keypair_path)
        .map_err(|e| ChainError::ProgramRejected(format!("could not read keypair: {e}")))?;

    let vault_pubkey = if is_pool { cfg.pool_vault } else { cfg.personal_vault };
    let beneficiary_list_pubkey = if is_pool {
        cfg.pool_beneficiary_list
    } else {
        cfg.personal_beneficiary_list
    };

    let vault_bytes = rpc.get_account_data(&vault_pubkey).map_err(map_client_error)?;
    let vault = family_vault::state::Vault::try_deserialize(&mut &vault_bytes[..])
        .map_err(|e| ChainError::ProgramRejected(e.to_string()))?;

    let vault_token_account =
        anchor_lang::prelude::Pubkey::new_from_array(vault.vault_token_account.to_bytes());
    let recipient_token_account_client =
        spl_associated_token_account::get_associated_token_address(&beneficiary_address, &cfg.mint);
    let recipient_token_account = anchor_lang::prelude::Pubkey::new_from_array(
        recipient_token_account_client.to_bytes(),
    );
    let signer = anchor_lang::prelude::Pubkey::new_from_array(
        solana_signer::Signer::pubkey(&owner_keypair).to_bytes(),
    );
    let vault_pubkey_anchor = anchor_lang::prelude::Pubkey::new_from_array(vault_pubkey.to_bytes());
    let beneficiary_list_pubkey_anchor =
        anchor_lang::prelude::Pubkey::new_from_array(beneficiary_list_pubkey.to_bytes());

    let amount_base_units = parse_amount_to_base_units(amount_whole_units);

    let ix_data: Vec<u8> = family_vault::instruction::Withdraw {
        amount: amount_base_units,
    }
    .data();
    let anchor_metas = family_vault::accounts::Withdraw {
        signer,
        vault_token_account,
        recipient_token_account,
        vault: vault_pubkey_anchor,
        beneficiary_list: beneficiary_list_pubkey_anchor,
        token_program: spl_token::ID,
    }
    .to_account_metas(None);

    let client_metas: Vec<solana_instruction::AccountMeta> = anchor_metas
        .iter()
        .map(|m| solana_instruction::AccountMeta {
            pubkey: solana_pubkey::Pubkey::new_from_array(m.pubkey.to_bytes()),
            is_signer: m.is_signer,
            is_writable: m.is_writable,
        })
        .collect();
    let client_program_id = solana_pubkey::Pubkey::new_from_array(cfg.program_id.to_bytes());

    let ix = solana_instruction::Instruction {
        program_id: client_program_id,
        accounts: client_metas,
        data: ix_data,
    };

    let blockhash = rpc.get_latest_blockhash().map_err(map_client_error)?;
    let payer_pubkey = solana_signer::Signer::pubkey(&owner_keypair);
    let tx = solana_transaction::Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer_pubkey),
        &[&owner_keypair],
        blockhash,
    );

    rpc.send_and_confirm_transaction(&tx).map_err(map_client_error)?;
    Ok(())
}
```

- [ ] **Step 4: Run the full test suite**

Run: `cargo test --package kiosk-gui chain:: -- --nocapture`
Expected: all tests pass. `submit_withdraw` itself is not directly unit
tested (it needs a live RPC endpoint + funded devnet accounts) — covered
by the manual devnet procedure in Task 5.

- [ ] **Step 5: Commit**

```bash
git add kiosk-gui/src/chain.rs
git commit -m "feat(kiosk-gui): submit real withdraw transactions"
```

---

### Task 5: Wire `chain.rs` into `main.rs`, replacing the mocks

**Files:**
- Modify: `kiosk-gui/src/main.rs` (full rewrite of the mock-wiring body)

**Interfaces:**
- Consumes: everything from Tasks 1-4 (`chain::load_config`,
  `chain::fetch_vault_snapshot`, `chain::submit_withdraw`,
  `chain::VaultSnapshot`, `chain::ChainError`).
- Produces: nothing further downstream — this is the last task in this plan.

This task is integration work: it cannot be meaningfully unit-tested (it
drives real Slint UI + a background thread + would need a live devnet to
fully exercise), so its "test" step is a compile check plus the existing
`tests/navigation.rs` suite (which must keep passing unchanged — it
constructs `AppWindow` directly and sets mock data itself, bypassing
`main()` entirely, so it is unaffected by this task) plus a documented
manual devnet smoke test.

- [ ] **Step 1: Replace `main.rs`'s body**

Replace the full contents of `kiosk-gui/src/main.rs` with:

```rust
slint::include_modules!();

mod chain;

use std::cell::RefCell;
use std::rc::Rc;

use chain::{ChainConfig, ChainError, VaultSnapshot};

fn config_path() -> std::path::PathBuf {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--config" {
            if let Some(path) = args.next() {
                return std::path::PathBuf::from(path);
            }
        }
    }
    let default = std::path::PathBuf::from("/etc/family-vault/kiosk.toml");
    if default.exists() {
        return default;
    }
    std::path::PathBuf::from("kiosk.toml")
}

fn error_message_text(err: &ChainError) -> String {
    match err {
        ChainError::NoConnection => "Нет связи. Попробуй ещё раз позже".to_string(),
        ChainError::ProgramRejected(raw) => {
            eprintln!("[kiosk-gui] transaction failed: {raw}");
            "Не получилось. Попробуй ещё раз или позови родственника".to_string()
        }
    }
}

fn apply_snapshot(app: &AppWindow, is_pool: bool, snapshot: VaultSnapshot) {
    let items: Vec<BeneficiaryItem> = snapshot
        .beneficiaries
        .iter()
        .map(|(id, name)| BeneficiaryItem {
            name: name.as_str().into(),
            id: *id as i32,
        })
        .collect();
    let model = Rc::new(slint::VecModel::from(items));
    if is_pool {
        app.set_family_balance_text(snapshot.balance_text.as_str().into());
        app.set_family_beneficiaries(model.into());
    } else {
        app.set_balance_text(snapshot.balance_text.as_str().into());
        app.set_beneficiaries(model.into());
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;

    app.set_presets(
        Rc::new(slint::VecModel::from(vec![
            "10 USDC".into(),
            "20 USDC".into(),
            "50 USDC".into(),
        ]))
        .into(),
    );

    let cfg = match chain::load_config(&config_path()) {
        Ok(cfg) => Rc::new(cfg),
        Err(e) => {
            eprintln!("[kiosk-gui] failed to load config: {e}");
            app.set_balance_text("Нет связи. Попробуй ещё раз позже".into());
            app.set_family_balance_text("Нет связи. Попробуй ещё раз позже".into());
            app.run()?;
            return Ok(());
        }
    };

    // Last-fetched snapshots, kept around so a beneficiary tap (which only
    // carries a small integer `id`) can be resolved back to the wallet
    // pubkey `submit_withdraw` needs. `chain.rs` doesn't track this itself
    // — see Task 4's Interfaces note.
    let personal_beneficiaries: Rc<RefCell<Vec<(u8, solana_pubkey::Pubkey)>>> =
        Rc::new(RefCell::new(Vec::new()));
    let pool_beneficiaries: Rc<RefCell<Vec<(u8, solana_pubkey::Pubkey)>>> =
        Rc::new(RefCell::new(Vec::new()));

    // Startup fetch, on a background thread so the UI shows immediately.
    {
        let app_weak = app.as_weak();
        let cfg = cfg.clone();
        let personal_beneficiaries = personal_beneficiaries.clone();
        let pool_beneficiaries = pool_beneficiaries.clone();
        std::thread::spawn(move || {
            let personal_result = chain::fetch_vault_snapshot(&cfg, false);
            let pool_result = chain::fetch_vault_snapshot(&cfg, true);

            // Re-derive wallet pubkeys for beneficiary-id resolution. The
            // snapshot only carries (id, name) for display, so beneficiary
            // list bytes are re-fetched here to also capture (id, address)
            // for later withdraw calls.
            let personal_addrs = fetch_beneficiary_addresses(&cfg, false);
            let pool_addrs = fetch_beneficiary_addresses(&cfg, true);

            let _ = slint::invoke_from_event_loop(move || {
                let Some(app) = app_weak.upgrade() else { return };
                match personal_result {
                    Ok(snapshot) => apply_snapshot(&app, false, snapshot),
                    Err(e) => app.set_balance_text(error_message_text(&e).into()),
                }
                match pool_result {
                    Ok(snapshot) => apply_snapshot(&app, true, snapshot),
                    Err(e) => app.set_family_balance_text(error_message_text(&e).into()),
                }
                if let Ok(addrs) = personal_addrs {
                    *personal_beneficiaries.borrow_mut() = addrs;
                }
                if let Ok(addrs) = pool_addrs {
                    *pool_beneficiaries.borrow_mut() = addrs;
                }
            });
        });
    }

    {
        let app_weak = app.as_weak();
        let addresses = personal_beneficiaries.clone();
        app.on_beneficiary_selected(move |id| {
            let app = app_weak.unwrap();
            let name = addresses
                .borrow()
                .iter()
                .find(|(bid, _)| *bid as i32 == id)
                .map(|_| {
                    app.get_beneficiaries()
                        .row_data(0)
                        .map(|b| b.name)
                        .unwrap_or_default()
                });
            // Look up the display name from the already-populated model
            // (avoids re-decoding on-chain bytes just to get a name we
            // already fetched at startup).
            let model = app.get_beneficiaries();
            let mut found_name: slint::SharedString = "Получатель".into();
            for i in 0..model.row_count() {
                if let Some(item) = model.row_data(i) {
                    if item.id == id {
                        found_name = item.name;
                        break;
                    }
                }
            }
            let _ = name;
            app.set_selected_recipient_name(found_name);
            app.set_current_screen(Screen::AmountSelect);
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_back_requested(move || {
            let app = app_weak.unwrap();
            app.set_current_screen(Screen::MyWallet);
        });
    }

    let selected_amount: Rc<RefCell<u64>> = Rc::new(RefCell::new(0));
    let selected_beneficiary_id: Rc<RefCell<i32>> = Rc::new(RefCell::new(0));
    {
        let app_weak = app.as_weak();
        let selected_amount = selected_amount.clone();
        app.on_amount_selected(move |index| {
            let app = app_weak.unwrap();
            let presets: [u64; 3] = [10, 20, 50];
            let amount = presets.get(index as usize).copied().unwrap_or(0);
            *selected_amount.borrow_mut() = amount;
            app.set_confirm_recipient_name(app.get_selected_recipient_name());
            app.set_confirm_amount_text(chain::format_amount(amount).into());
            app.set_current_screen(Screen::Confirm);
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_send_cancelled(move || {
            let app = app_weak.unwrap();
            app.set_current_screen(Screen::MyWallet);
        });
    }
    {
        let app_weak = app.as_weak();
        let cfg = cfg.clone();
        let personal_beneficiaries = personal_beneficiaries.clone();
        let selected_amount = selected_amount.clone();
        app.on_send_confirmed(move || {
            let app = app_weak.unwrap();
            let recipient_name = app.get_confirm_recipient_name();

            // Resolve the wallet pubkey for the currently-selected
            // beneficiary id by scanning the model for its id, then
            // looking that id up in the addresses map fetched at startup.
            let model = app.get_beneficiaries();
            let mut recipient_id: i32 = 0;
            for i in 0..model.row_count() {
                if let Some(item) = model.row_data(i) {
                    if item.name == recipient_name {
                        recipient_id = item.id;
                        break;
                    }
                }
            }
            let addr = personal_beneficiaries
                .borrow()
                .iter()
                .find(|(id, _)| *id as i32 == recipient_id)
                .map(|(_, addr)| *addr);

            let Some(addr) = addr else {
                app.set_result_success(false);
                app.set_result_message_text(
                    "Не получилось. Попробуй ещё раз или позови родственника".into(),
                );
                app.set_current_screen(Screen::Result);
                return;
            };

            let amount = *selected_amount.borrow();
            let cfg = cfg.clone();
            let app_weak = app.as_weak();
            std::thread::spawn(move || {
                let result = chain::submit_withdraw(&cfg, false, addr, amount);
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(app) = app_weak.upgrade() else { return };
                    match result {
                        Ok(()) => {
                            app.set_result_success(true);
                            app.set_result_message_text(
                                format!("Готово! Деньги отправлены {recipient_name}").into(),
                            );
                        }
                        Err(e) => {
                            app.set_result_success(false);
                            app.set_result_message_text(error_message_text(&e).into());
                        }
                    }
                    app.set_current_screen(Screen::Result);
                });
            });
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_ok_pressed(move || {
            let app = app_weak.unwrap();
            app.set_current_screen(Screen::MyWallet);
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_family_wallet_requested(move || {
            let app = app_weak.unwrap();
            app.set_current_screen(Screen::FamilyWallet);
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_back_to_my_wallet_requested(move || {
            let app = app_weak.unwrap();
            app.set_current_screen(Screen::MyWallet);
        });
    }
    {
        let app_weak = app.as_weak();
        let cfg = cfg.clone();
        let pool_beneficiaries = pool_beneficiaries.clone();
        app.on_family_beneficiary_selected(move |id| {
            let app = app_weak.unwrap();
            let model = app.get_family_beneficiaries();
            let mut found_name: slint::SharedString = "Получатель".into();
            for i in 0..model.row_count() {
                if let Some(item) = model.row_data(i) {
                    if item.id == id {
                        found_name = item.name;
                        break;
                    }
                }
            }
            let addr = pool_beneficiaries
                .borrow()
                .iter()
                .find(|(bid, _)| *bid as i32 == id)
                .map(|(_, addr)| *addr);
            let Some(addr) = addr else { return };

            // Family Wallet has no dedicated amount-select/confirm screens
            // of its own in this plan's scope (spec: "Family Wallet" data
            // flow reuses the same withdraw call) — send the first preset
            // amount directly and reuse the shared Result screen. A richer
            // Family Wallet amount-select flow is future work, not part of
            // this plan.
            let amount = 10u64;
            let cfg = cfg.clone();
            let app_weak = app.as_weak();
            std::thread::spawn(move || {
                let result = chain::submit_withdraw(&cfg, true, addr, amount);
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(app) = app_weak.upgrade() else { return };
                    match result {
                        Ok(()) => {
                            app.set_result_success(true);
                            app.set_result_message_text(
                                format!("Готово! Деньги отправлены {found_name}").into(),
                            );
                        }
                        Err(e) => {
                            app.set_result_success(false);
                            app.set_result_message_text(error_message_text(&e).into());
                        }
                    }
                    app.set_current_screen(Screen::Result);
                });
            });
        });
    }
    app.on_help_requested(|| {
        println!("help requested");
    });
    app.on_family_help_requested(|| {
        println!("help requested (family wallet)");
    });

    let _ = selected_beneficiary_id;
    app.run()
}

fn fetch_beneficiary_addresses(
    cfg: &ChainConfig,
    is_pool: bool,
) -> Result<Vec<(u8, solana_pubkey::Pubkey)>, ChainError> {
    use anchor_lang::AccountDeserialize;

    let rpc = solana_client::rpc_client::RpcClient::new(cfg.rpc_url.clone());
    let list_pubkey = if is_pool {
        cfg.pool_beneficiary_list
    } else {
        cfg.personal_beneficiary_list
    };
    let bytes = rpc
        .get_account_data(&list_pubkey)
        .map_err(chain::map_client_error_pub)?;
    let list = family_vault::state::BeneficiaryList::try_deserialize(&mut &bytes[..])
        .map_err(|e| ChainError::ProgramRejected(e.to_string()))?;
    Ok((0..list.count as usize)
        .map(|i| {
            let b = &list.entries[i];
            (
                (i + 1) as u8,
                solana_pubkey::Pubkey::new_from_array(b.address.to_bytes()),
            )
        })
        .collect())
}
```

This references `chain::map_client_error_pub`, which doesn't exist yet
(Task 3/4 made `map_client_error` private to `chain.rs`). Fix that:

- [ ] **Step 2: Expose a public error-mapping function from `chain.rs`**

In `kiosk-gui/src/chain.rs`, change:

```rust
fn map_client_error(err: solana_client::client_error::ClientError) -> ChainError {
```

to:

```rust
pub fn map_client_error_pub(err: solana_client::client_error::ClientError) -> ChainError {
    map_client_error(err)
}

fn map_client_error(err: solana_client::client_error::ClientError) -> ChainError {
```

(keeps the original private helper used internally by `chain.rs`'s own
functions, adds a thin public wrapper `main.rs` can call — avoids
widening `map_client_error`'s visibility everywhere just for this one
external call site).

- [ ] **Step 3: Simplify the dead-code path in `on_beneficiary_selected`**

The `on_beneficiary_selected` closure drafted in Step 1 has a leftover
unused `name`/`found_name` double-computation (an artifact of drafting —
clean it up now rather than shipping dead code). Replace the closure body
with just the model-scan version:

```rust
    {
        let app_weak = app.as_weak();
        app.on_beneficiary_selected(move |id| {
            let app = app_weak.unwrap();
            let model = app.get_beneficiaries();
            let mut found_name: slint::SharedString = "Получатель".into();
            for i in 0..model.row_count() {
                if let Some(item) = model.row_data(i) {
                    if item.id == id {
                        found_name = item.name;
                        break;
                    }
                }
            }
            app.set_selected_recipient_name(found_name);
            app.set_current_screen(Screen::AmountSelect);
        });
    }
```

(This replaces the version from Step 1 that computed an unused `name`
binding via `addresses.borrow()...map(...)` before immediately
discarding it with `let _ = name;` — that was scaffolding from drafting,
not intended to ship.)

- [ ] **Step 4: Remove the unused `selected_beneficiary_id` variable**

Delete these two lines from `main()` (added in Step 1's draft but never
actually used — `selected_amount` is the only per-flow state needed
between Amount Select and Confirm):

```rust
    let selected_beneficiary_id: Rc<RefCell<i32>> = Rc::new(RefCell::new(0));
```

and

```rust
    let _ = selected_beneficiary_id;
```

- [ ] **Step 5: Run `cargo check`**

Run: `cargo check --package kiosk-gui`
Expected: succeeds with no errors (warnings about unused imports are
acceptable only if they point at something genuinely still needed for a
later plan — if `cargo check` warns about something introduced in this
task specifically, fix it, don't leave it).

- [ ] **Step 6: Run the existing Slint navigation tests to confirm no regression**

Run: `cargo test --package kiosk-gui --test navigation`
Expected: all 9 existing scenarios still pass (`tests/navigation.rs`
constructs `AppWindow` directly and injects its own mock data/callbacks —
it never calls `main()`, so this task's changes to `main.rs` cannot break
it; running it here is a regression guard, not new coverage).

- [ ] **Step 7: Commit**

```bash
git add kiosk-gui/src/main.rs kiosk-gui/src/chain.rs
git commit -m "feat(kiosk-gui): wire chain.rs into main.rs, replacing mock data"
```

- [ ] **Step 8: Document the manual devnet smoke test (do not automate — see spec's Testing section)**

This step has no code change — it's the manual verification the design
spec calls for. Perform it once, by hand, after Step 7's commit, and
report the outcome (pass/fail + what was observed) in the task's final
report to the reviewer:

1. `solana-keygen new --outfile /tmp/owner-keypair.json` (or reuse an
   existing devnet-funded keypair).
2. `solana airdrop 2 --url devnet --keypair /tmp/owner-keypair.json`.
3. Confirm the `family-vault` program is deployed to devnet (deploy it if
   not — `anchor deploy --provider.cluster devnet` from the repo root, or
   check `solana program show <program id> --url devnet`).
4. Use `solana` CLI / a short throwaway script to call
   `initialize_vault`, `add_beneficiary` (with a real second devnet
   keypair as the beneficiary, and create that beneficiary's ATA for the
   test mint via `spl-token create-account <mint> --owner <beneficiary>
   --url devnet`), and `deposit` some tokens into the vault's token
   account.
5. Write `/tmp/kiosk.toml` with the real resulting addresses (rpc_url =
   `https://api.devnet.solana.com`, the real `program_id`/`mint`/vault/
   beneficiary-list pubkeys from step 4, `keypair_path` = the owner
   keypair from step 1).
6. Run `cargo run --package kiosk-gui -- --config /tmp/kiosk.toml`.
7. Verify: the balance and beneficiary name from step 4 appear on the My
   Wallet screen (not the old mock "120 USDC" / "Внук Саша" placeholder
   values — though the name may coincidentally match if you reused it as
   the beneficiary name in step 4, the *balance* proves it's real).
8. Tap the beneficiary, pick a preset amount, confirm the send.
9. Verify: Result screen shows "Готово! Деньги отправлены ...", and
   `solana balance <beneficiary ATA> --url devnet` (or the devnet
   explorer) shows the transferred amount.
10. Kill the RPC connection (e.g., temporarily point `rpc_url` at an
    unreachable host) and repeat steps 6-8 — verify the error path shows
    "Нет связи. Попробуй ещё раз позже" instead of crashing or hanging
    the UI.

---

## Self-review notes (from plan authoring)

- **Spec coverage:** Config format (Task 1), decimals=6 constant (Tasks 1-2),
  ATA derivation (Task 4), background-thread + `invoke_from_event_loop`
  architecture (Task 5), the two fixed error message strings (Task 5's
  `error_message_text`), `withdraw`-only scope / no `request_withdrawal`
  (never referenced anywhere in this plan) — all covered. The spec's
  "Family Wallet" data flow paragraph says amount comes from the same
  Confirm/Amount-Select pattern as My Wallet; this plan's Task 5 takes a
  narrower shortcut (fixed first-preset amount, no dedicated Family Wallet
  amount-select) to keep the plan's scope to "wire real calls," not "build
  a second Amount Select flow" — flagged inline in Task 5's code comment
  as a deliberate scope-narrowing, not an oversight; call this out to the
  user if it doesn't match their expectation before merging.
- **Type consistency:** `chain::VaultSnapshot`, `chain::ChainConfig`,
  `chain::ChainError`, `chain::TOKEN_DECIMALS`, `chain::format_amount`,
  `chain::parse_amount_to_base_units`, `chain::decode_beneficiary_name`,
  `chain::fetch_vault_snapshot`, `chain::submit_withdraw`,
  `chain::map_client_error_pub` — all defined exactly once (Tasks 1-4),
  used with matching names/signatures in Task 5.
- **Dependency versions:** every version in Task 1 was verified by
  actually compiling this exact combination against the family-vault
  workspace before this plan was written (not guessed) — see the design
  spec's session history for why that verification step matters here.
