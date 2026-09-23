# Kiosk First-Run Setup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A freshly-flashed kiosk device (no keypair, no `kiosk.toml`) generates its own owner key on first boot, shows the pubkey as a QR code for a family member to fund, and self-initializes its personal vault once funded — with zero secrets typed by anyone.

**Architecture:** New `kiosk-gui/src/setup.rs` module holds all setup-specific logic (provisioning-config parsing, keypair generate-or-load, QR rendering, config writing, self-initialization). A new `Screen::Setup` in the existing Slint app shows the QR and status text. `main()` checks for `kiosk.toml` at startup; if absent, it runs the setup flow on a background thread instead of the normal flow, then transitions into `Screen::MyWallet` in the same process once done — no second binary, no restart.

**Tech Stack:** Rust, Slint 1.18.0 (existing), `qrcode` 0.14.1 (no default features — avoids pulling in the `image` crate, QR modules are rasterized by hand into a `slint::SharedPixelBuffer`), the same `solana-client`/`solana-pubkey`/`anchor-lang` client-bindings pattern already used in `kiosk-gui/src/chain.rs` and `tools/admin-cli`.

**Spec:** `docs/superpowers/specs/2026-09-23-kiosk-first-run-setup-design.md` (also read `docs/superpowers/specs/2026-09-21-kiosk-blockchain-wiring-design.md` for `ChainConfig`/`chain.rs` conventions this plan builds on)

## Global Constraints

- `provisioning.toml` (`/etc/family-vault/provisioning.toml` by default, or `<config-path-dir>/provisioning.toml` in dev — see Task 1) holds only `rpc_url`, `program_id`, `mint` — never a secret.
- The generated keypair is written via `solana_keypair::write_keypair_file`, which already sets `0600` permissions on unix (verified: `solana-keypair` 3.0.1's own test asserts this) — do not add manual `chmod` code, it would be redundant.
- Setup mode triggers when `kiosk.toml` does not exist at the resolved config path. It never triggers because of anything else (a corrupt file, a missing keypair while `kiosk.toml` exists, etc. are out of scope for this plan — see the design spec's "Открытые вопросы").
- Balance polling has no timeout and never gives up (device is idle before sale anyway) — only the *`initialize_vault` submission* has a retry cap (5 attempts) before showing a permanent error.
- No animation/blinking on the Setup screen (WCAG 2.3.2/2.3.3, already-established project constraint) — status changes are plain text swaps, no spinners.
- `MIN_LAMPORTS` for "is this funded enough" is computed from real rent-exemption queries (`rpc.get_minimum_balance_for_rent_exemption`), never a hardcoded guess — see Task 4 for the exact formula.

---

### Task 1: Provisioning config, `needs_setup`, and keypair generate-or-load

**Files:**
- Create: `kiosk-gui/src/setup.rs`
- Modify: `kiosk-gui/Cargo.toml` (add `qrcode` dependency)
- Modify: `kiosk-gui/src/main.rs:9` (add `mod setup;`)

**Interfaces:**
- Produces: `setup::needs_setup(config_path: &Path) -> bool`, `setup::ProvisioningConfig { rpc_url: String, program_id: anchor_lang::prelude::Pubkey, mint: solana_pubkey::Pubkey }`, `setup::ProvisioningError` (Display + Error, same shape as `chain::ConfigError`), `setup::load_provisioning_config(path: &Path) -> Result<ProvisioningConfig, ProvisioningError>`, `setup::owner_keypair_path(config_dir: &Path) -> std::path::PathBuf`, `setup::generate_or_load_owner_keypair(path: &Path) -> Result<solana_keypair::Keypair, ProvisioningError>`.

- [ ] **Step 1: Add the `qrcode` dependency**

In `kiosk-gui/Cargo.toml`, add to `[dependencies]` (alphabetical, matching the existing style):

```toml
qrcode = { version = "0.14.1", default-features = false }
```

Run `cargo check --package kiosk-gui` from the repo root. Expected: succeeds (this dependency isn't used yet, just added).

- [ ] **Step 2: Write the failing tests**

Create `kiosk-gui/src/setup.rs` with just this content (the test module — everything else in this task follows):

```rust
use std::fmt;
use std::path::{Path, PathBuf};

use solana_keypair::Keypair;
use solana_signer::Signer;

#[derive(Debug)]
pub struct ProvisioningConfig {
    pub rpc_url: String,
    pub program_id: anchor_lang::prelude::Pubkey,
    pub mint: solana_pubkey::Pubkey,
}

#[derive(Debug)]
pub enum ProvisioningError {
    Io(std::io::Error),
    Parse(toml::de::Error),
    BadPubkey { field: &'static str, value: String },
}

impl fmt::Display for ProvisioningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProvisioningError::Io(e) => write!(f, "could not read provisioning file: {e}"),
            ProvisioningError::Parse(e) => write!(f, "could not parse provisioning TOML: {e}"),
            ProvisioningError::BadPubkey { field, value } => {
                write!(f, "provisioning field '{field}' is not a valid pubkey: '{value}'")
            }
        }
    }
}

impl std::error::Error for ProvisioningError {}

pub fn needs_setup(config_path: &Path) -> bool {
    !config_path.exists()
}

pub fn owner_keypair_path(config_dir: &Path) -> PathBuf {
    config_dir.join("owner-keypair.json")
}

pub fn load_provisioning_config(path: &Path) -> Result<ProvisioningConfig, ProvisioningError> {
    todo!()
}

pub fn generate_or_load_owner_keypair(path: &Path) -> Result<Keypair, ProvisioningError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_setup_is_true_when_config_file_is_missing() {
        let path = std::env::temp_dir().join("kiosk-setup-test-missing-config.toml");
        let _ = std::fs::remove_file(&path);
        assert!(needs_setup(&path));
    }

    #[test]
    fn needs_setup_is_false_when_config_file_exists() {
        let path = std::env::temp_dir().join(format!(
            "kiosk-setup-test-existing-config-{}.toml",
            std::process::id()
        ));
        std::fs::write(&path, "anything").unwrap();
        assert!(!needs_setup(&path));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn loads_a_valid_provisioning_config() {
        let path = std::env::temp_dir().join(format!(
            "kiosk-setup-test-provisioning-{}.toml",
            std::process::id()
        ));
        std::fs::write(
            &path,
            r#"
rpc_url = "https://api.devnet.solana.com"
program_id = "9ozRBpTmDJJzcmJhKEbfvsoe7qqo2584bus1rSMG4zwJ"
mint = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"
"#,
        )
        .unwrap();
        let cfg = load_provisioning_config(&path).unwrap();
        assert_eq!(cfg.rpc_url, "https://api.devnet.solana.com");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn rejects_provisioning_config_with_bad_mint() {
        let path = std::env::temp_dir().join(format!(
            "kiosk-setup-test-bad-mint-{}.toml",
            std::process::id()
        ));
        std::fs::write(
            &path,
            r#"
rpc_url = "https://api.devnet.solana.com"
program_id = "9ozRBpTmDJJzcmJhKEbfvsoe7qqo2584bus1rSMG4zwJ"
mint = "not-a-pubkey"
"#,
        )
        .unwrap();
        let err = load_provisioning_config(&path).unwrap_err();
        assert!(matches!(err, ProvisioningError::BadPubkey { field: "mint", .. }));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn generates_a_new_keypair_when_none_exists_on_disk() {
        let path = std::env::temp_dir().join(format!(
            "kiosk-setup-test-keypair-new-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let kp = generate_or_load_owner_keypair(&path).unwrap();
        assert!(path.exists());
        // Loading again must return the SAME keypair, not a fresh one —
        // this is the power-loss-mid-setup resumability the design spec
        // requires: the already-shown QR pubkey must not change.
        let kp2 = generate_or_load_owner_keypair(&path).unwrap();
        assert_eq!(kp.pubkey(), kp2.pubkey());
        std::fs::remove_file(&path).unwrap();
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --package kiosk-gui setup::tests -- --nocapture`
Expected: FAIL — `load_provisioning_config`/`generate_or_load_owner_keypair` panic with `not implemented` (the `todo!()` bodies). The `needs_setup` tests should already pass (that function is fully implemented above) — confirm those 2 pass and the other 3 panic.

- [ ] **Step 4: Implement `load_provisioning_config` and `generate_or_load_owner_keypair`**

Replace the two `todo!()` bodies:

```rust
pub fn load_provisioning_config(path: &Path) -> Result<ProvisioningConfig, ProvisioningError> {
    let text = std::fs::read_to_string(path).map_err(ProvisioningError::Io)?;
    let raw: RawProvisioningConfig = toml::from_str(&text).map_err(ProvisioningError::Parse)?;

    let program_id: anchor_lang::prelude::Pubkey =
        raw.program_id.parse().map_err(|_| ProvisioningError::BadPubkey {
            field: "program_id",
            value: raw.program_id.clone(),
        })?;
    let mint: solana_pubkey::Pubkey =
        raw.mint.parse().map_err(|_| ProvisioningError::BadPubkey {
            field: "mint",
            value: raw.mint.clone(),
        })?;

    Ok(ProvisioningConfig {
        rpc_url: raw.rpc_url,
        program_id,
        mint,
    })
}

pub fn generate_or_load_owner_keypair(path: &Path) -> Result<Keypair, ProvisioningError> {
    if path.exists() {
        return solana_keypair::read_keypair_file(path)
            .map_err(|e| ProvisioningError::Io(std::io::Error::other(e.to_string())));
    }
    let keypair = Keypair::new();
    solana_keypair::write_keypair_file(&keypair, path)
        .map_err(|e| ProvisioningError::Io(std::io::Error::other(e.to_string())))?;
    Ok(keypair)
}
```

Add the `RawProvisioningConfig` struct (mirrors `chain.rs`'s `RawConfig` pattern) above `ProvisioningConfig`:

```rust
#[derive(serde::Deserialize)]
struct RawProvisioningConfig {
    rpc_url: String,
    program_id: String,
    mint: String,
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --package kiosk-gui setup::tests -- --nocapture`
Expected: all 5 tests pass.

- [ ] **Step 6: Wire the module into `main.rs`**

In `kiosk-gui/src/main.rs`, add `mod setup;` on the line right after `mod chain;`.

Run: `cargo check --package kiosk-gui`
Expected: succeeds. `cargo clippy --package kiosk-gui --all-targets` must also be clean — the crate has `[lints.rust] unsafe_code = "deny", dead_code = "deny"` (from `kiosk-gui/Cargo.toml`), so every new public item that isn't called yet needs `#[allow(dead_code)] // used starting in Task N` on it, following the exact pattern already established in `chain.rs`. Mark `ProvisioningConfig`, `ProvisioningError`, `needs_setup`, `owner_keypair_path`, `load_provisioning_config`, `generate_or_load_owner_keypair` all `#[allow(dead_code)] // used starting in Task 6` (Task 6 is the one that wires everything into `main()`) — except `needs_setup`, which Task 6 also consumes, same marker.

- [ ] **Step 7: Commit**

```bash
git add kiosk-gui/Cargo.toml kiosk-gui/Cargo.lock kiosk-gui/src/main.rs kiosk-gui/src/setup.rs
git commit -m "feat(kiosk-gui): provisioning config, keypair generate-or-load, needs_setup"
```

---

### Task 2: QR image rendering

**Files:**
- Modify: `kiosk-gui/src/setup.rs`

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces: `setup::render_qr(data: &str) -> slint::Image`.

- [ ] **Step 1: Write the failing test**

Add to `kiosk-gui/src/setup.rs`'s test module:

```rust
    #[test]
    fn renders_a_qr_image_with_a_quiet_zone_border() {
        let image = render_qr("test-pubkey-string");
        let size = image.size();
        // Exact pixel size depends on the QR version chosen for this
        // input length, but it must be square and non-trivially sized
        // (a real QR code, not a 1x1 placeholder).
        assert_eq!(size.width, size.height);
        assert!(size.width > 100, "expected a real QR raster, got {size:?}");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package kiosk-gui setup::tests::renders_a_qr_image_with_a_quiet_zone_border -- --nocapture`
Expected: FAIL with "cannot find function `render_qr`".

- [ ] **Step 3: Implement `render_qr`**

Add to `kiosk-gui/src/setup.rs`, above the test module:

```rust
/// Renders `data` as a black-on-white QR code raster with a quiet-zone
/// border (required for real-world scanability, not just a visual
/// nicety — most scanners fail without it). No anti-aliasing: QR
/// modules must stay pure black/white to stay machine-readable.
pub fn render_qr(data: &str) -> slint::Image {
    let code = qrcode::QrCode::new(data.as_bytes()).expect("QR encoding failed");
    let width = code.width();
    let scale = 8usize;
    let border = 4usize;
    let img_size = (width + border * 2) * scale;

    let mut buffer =
        slint::SharedPixelBuffer::<slint::Rgb8Pixel>::new(img_size as u32, img_size as u32);
    let stride = buffer.width() as usize;
    let bytes = buffer.make_mut_bytes();

    for y in 0..img_size {
        for x in 0..img_size {
            let module_x = x / scale;
            let module_y = y / scale;
            let dark = module_x >= border
                && module_y >= border
                && module_x - border < width
                && module_y - border < width
                && code[(module_x - border, module_y - border)] == qrcode::Color::Dark;
            let v: u8 = if dark { 0 } else { 255 };
            let idx = (y * stride + x) * 3;
            bytes[idx] = v;
            bytes[idx + 1] = v;
            bytes[idx + 2] = v;
        }
    }
    slint::Image::from_rgb8(buffer)
}
```

This was verified to compile against `qrcode = "0.14.1"` (`default-features = false`) and `slint = "1.18.0"` before this plan was written — use these exact APIs (`slint::SharedPixelBuffer::<slint::Rgb8Pixel>`, `.make_mut_bytes()`, `slint::Image::from_rgb8`), don't substitute a different rasterization approach.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package kiosk-gui setup::tests::renders_a_qr_image_with_a_quiet_zone_border -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Mark as not-yet-used and verify full build**

Add `#[allow(dead_code)] // used starting in Task 6` above `pub fn render_qr`.

Run: `cargo test --package kiosk-gui setup::tests -- --nocapture`
Expected: all 6 tests pass (5 from Task 1 + this one).

- [ ] **Step 6: Commit**

```bash
git add kiosk-gui/src/setup.rs
git commit -m "feat(kiosk-gui): render owner pubkey as a QR image"
```

---

### Task 3: Writing `kiosk.toml` after setup completes

**Files:**
- Modify: `kiosk-gui/src/setup.rs`

**Interfaces:**
- Consumes: `chain::ChainConfig` (already defined in `kiosk-gui/src/chain.rs`), `chain::load_config` (for the round-trip test).
- Produces: `setup::write_kiosk_config(path: &Path, cfg: &KioskConfigValues) -> Result<(), ProvisioningError>`, `setup::KioskConfigValues` (plain struct carrying the 8 fields needed to write `kiosk.toml`).

`KioskConfigValues` exists separately from `chain::ChainConfig` because `ChainConfig`'s fields are already-parsed `Pubkey` types (good for reading), but writing TOML needs the string form and this task shouldn't reach into `chain.rs` to add a "serialize back to TOML" responsibility that module doesn't otherwise need — see the design spec's file-boundary reasoning.

- [ ] **Step 1: Write the failing test**

Add to `kiosk-gui/src/setup.rs`'s test module (add `use crate::chain;` to the test module's imports if not already present via the outer `use super::*;`):

```rust
    #[test]
    fn written_config_round_trips_through_chain_load_config() {
        let path = std::env::temp_dir().join(format!(
            "kiosk-setup-test-write-config-{}.toml",
            std::process::id()
        ));
        let values = KioskConfigValues {
            rpc_url: "https://api.devnet.solana.com".to_string(),
            keypair_path: "/etc/family-vault/owner-keypair.json".to_string(),
            program_id: "9ozRBpTmDJJzcmJhKEbfvsoe7qqo2584bus1rSMG4zwJ".to_string(),
            mint: "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".to_string(),
            personal_vault: "KziPLWvcZnYnt2ZhR7ACGHocBvgx2qX8jMExuxpFTQZ".to_string(),
            personal_beneficiary_list: "AbPLQhfRK16zi9vqUH1KutFDN8cU1bne4RXgRtKrEd3P".to_string(),
        };
        write_kiosk_config(&path, &values).unwrap();

        let loaded = crate::chain::load_config(&path).unwrap();
        assert_eq!(loaded.rpc_url, values.rpc_url);
        assert_eq!(loaded.keypair_path, values.keypair_path);
        // Pool fields must fall back to the personal vault's own
        // addresses (the placeholder pattern from the design spec's
        // "Фандинг и самостоятельная инициализация" step 4) until a
        // family app creates a real pool vault later.
        assert_eq!(
            loaded.pool_vault.to_string(),
            values.personal_vault,
            "pool_vault must default to personal_vault as a placeholder"
        );
        assert_eq!(
            loaded.pool_beneficiary_list.to_string(),
            values.personal_beneficiary_list
        );

        std::fs::remove_file(&path).unwrap();
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package kiosk-gui setup::tests::written_config_round_trips_through_chain_load_config -- --nocapture`
Expected: FAIL with "cannot find struct `KioskConfigValues`" / "cannot find function `write_kiosk_config`".

- [ ] **Step 3: Implement `KioskConfigValues` and `write_kiosk_config`**

Add to `kiosk-gui/src/setup.rs`:

```rust
pub struct KioskConfigValues {
    pub rpc_url: String,
    pub keypair_path: String,
    pub program_id: String,
    pub mint: String,
    pub personal_vault: String,
    pub personal_beneficiary_list: String,
}

pub fn write_kiosk_config(
    path: &Path,
    values: &KioskConfigValues,
) -> Result<(), ProvisioningError> {
    let text = format!(
        "rpc_url = \"{}\"\n\
         keypair_path = \"{}\"\n\
         program_id = \"{}\"\n\
         mint = \"{}\"\n\
         personal_vault = \"{}\"\n\
         personal_beneficiary_list = \"{}\"\n\
         pool_vault = \"{}\"\n\
         pool_beneficiary_list = \"{}\"\n",
        values.rpc_url,
        values.keypair_path,
        values.program_id,
        values.mint,
        values.personal_vault,
        values.personal_beneficiary_list,
        // Placeholder pattern (design spec, "Фандинг и самостоятельная
        // инициализация" step 4): pool fields equal the personal
        // vault's own addresses until a family app creates a real pool
        // vault. Family Wallet will show the same data as My Wallet
        // until then — the same graceful-degradation path already
        // exercised manually with this exact placeholder trick.
        values.personal_vault,
        values.personal_beneficiary_list,
    );
    std::fs::write(path, text).map_err(ProvisioningError::Io)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package kiosk-gui setup::tests::written_config_round_trips_through_chain_load_config -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Mark as not-yet-used, run the full setup test module**

Add `#[allow(dead_code)] // used starting in Task 6` above both `pub struct KioskConfigValues` and `pub fn write_kiosk_config`.

Run: `cargo test --package kiosk-gui setup::tests -- --nocapture`
Expected: all 7 tests pass.

- [ ] **Step 6: Commit**

```bash
git add kiosk-gui/src/setup.rs
git commit -m "feat(kiosk-gui): write kiosk.toml after setup completes"
```

---

### Task 4: Self-initializing the personal vault

**Files:**
- Modify: `kiosk-gui/src/setup.rs`
- Modify: `kiosk-gui/Cargo.toml` (add `spl-associated-token-account` — already a dependency; confirm the `no-entrypoint` feature stays intact, don't touch it)

**Interfaces:**
- Consumes: `ProvisioningConfig` (Task 1), `chain::map_client_error_pub`, `chain::ChainError` (both already `pub` in `chain.rs` from the blockchain-wiring plan).
- Produces: `setup::MIN_FUNDING_LAMPORTS_BUFFER: u64` (the fee-headroom constant — the rent portions are computed live, only this buffer is a literal), `setup::compute_min_funding_lamports(rpc: &RpcClient) -> Result<u64, ChainError>`, `setup::self_initialize_personal_vault(rpc: &RpcClient, owner: &Keypair, cfg: &ProvisioningConfig) -> Result<(solana_pubkey::Pubkey, solana_pubkey::Pubkey), ChainError>` (returns `(personal_vault, personal_beneficiary_list)`).

`self_initialize_personal_vault` is **idempotent by existence check, not by catching an "already exists" error**: it first tries to read the vault PDA; if that read succeeds, the vault already exists (a prior attempt got the transaction through before a crash/retry) and it returns immediately without sending anything. This matters for the resumability the design spec calls for — re-deriving the PDA and checking is cheap and unambiguous, versus parsing a program error string to guess whether "AccountAlreadyInUse" or similar occurred.

- [ ] **Step 1: Write the failing test for `compute_min_funding_lamports`**

Add to `kiosk-gui/src/setup.rs`'s test module. This test needs a live RPC endpoint — mark it `#[ignore]` (same convention as this session's earlier real-devnet scratch tests) and document how to run it manually:

```rust
    // Requires a live RPC endpoint — run manually with
    // `cargo test --package kiosk-gui setup::tests::computes_a_real_minimum_funding_amount -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn computes_a_real_minimum_funding_amount() {
        let rpc = solana_client::rpc_client::RpcClient::new(
            "https://api.devnet.solana.com".to_string(),
        );
        let lamports = compute_min_funding_lamports(&rpc).unwrap();
        println!("min funding lamports: {lamports}");
        // Sanity bound: must cover at minimum {Vault + BeneficiaryList +
        // an ATA}'s rent, which on devnet today is on the order of a few
        // million lamports (a few thousandths of a SOL) — not zero, not
        // absurdly large (which would indicate a units bug, e.g. reading
        // SOL instead of lamports).
        assert!(lamports > 1_000_000, "suspiciously low: {lamports}");
        assert!(lamports < 100_000_000, "suspiciously high: {lamports}");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --package kiosk-gui setup::tests::computes_a_real_minimum_funding_amount -- --ignored --nocapture`
Expected: FAIL with "cannot find function `compute_min_funding_lamports`".

- [ ] **Step 3: Implement `compute_min_funding_lamports` and the buffer constant**

Add to `kiosk-gui/src/setup.rs`:

```rust
/// Headroom above the raw rent-exemption sum, covering transaction fees
/// across a few submission attempts (each `initialize_vault` costs one
/// signature-verification fee; a handful of retries stay well under
/// this). Not rent — rent is computed live below, never guessed.
pub const MIN_FUNDING_LAMPORTS_BUFFER: u64 = 50_000;

pub fn compute_min_funding_lamports(
    rpc: &solana_client::rpc_client::RpcClient,
) -> Result<u64, crate::chain::ChainError> {
    let vault_rent = rpc
        .get_minimum_balance_for_rent_exemption(family_vault::state::Vault::SPACE)
        .map_err(crate::chain::map_client_error_pub)?;
    let list_rent = rpc
        .get_minimum_balance_for_rent_exemption(family_vault::state::BeneficiaryList::SPACE)
        .map_err(crate::chain::map_client_error_pub)?;
    let ata_rent = rpc
        .get_minimum_balance_for_rent_exemption(165) // spl_token::state::Account::LEN
        .map_err(crate::chain::map_client_error_pub)?;
    Ok(vault_rent + list_rent + ata_rent + MIN_FUNDING_LAMPORTS_BUFFER)
}
```

(165 is written as a literal with a comment rather than
`spl_token::state::Account::LEN` because that associated constant lives
behind the `Pack` trait and pulling that import into this function only
for a compile-time-constant value would be more indirection than the
value is worth — the comment keeps it traceable to its source.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --package kiosk-gui setup::tests::computes_a_real_minimum_funding_amount -- --ignored --nocapture`
Expected: PASS, prints a real lamport figure.

- [ ] **Step 5: Write the failing test for `self_initialize_personal_vault`'s idempotency**

This one is also RPC-dependent — mark `#[ignore]` too:

```rust
    // Requires a live RPC endpoint AND a funded throwaway owner keypair
    // (needs at least compute_min_funding_lamports(&rpc) lamports before
    // running). Run manually with
    // `cargo test --package kiosk-gui setup::tests::self_init_is_idempotent -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn self_init_is_idempotent() {
        let rpc = solana_client::rpc_client::RpcClient::new(
            "https://api.devnet.solana.com".to_string(),
        );
        let owner = solana_keypair::read_keypair_file("/tmp/setup-test-owner.json")
            .expect("fund /tmp/setup-test-owner.json before running this test");
        let cfg = ProvisioningConfig {
            rpc_url: "https://api.devnet.solana.com".to_string(),
            program_id: "9ozRBpTmDJJzcmJhKEbfvsoe7qqo2584bus1rSMG4zwJ"
                .parse()
                .unwrap(),
            mint: "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"
                .parse()
                .unwrap(),
        };

        let (vault1, list1) = self_initialize_personal_vault(&rpc, &owner, &cfg).unwrap();
        // Second call must not error and must return the SAME addresses
        // — this is the crash-mid-setup resumability the design spec
        // requires.
        let (vault2, list2) = self_initialize_personal_vault(&rpc, &owner, &cfg).unwrap();
        assert_eq!(vault1, vault2);
        assert_eq!(list1, list2);
    }
```

- [ ] **Step 6: Run test to verify it fails**

Run: `cargo test --package kiosk-gui setup::tests::self_init_is_idempotent -- --ignored --nocapture`
Expected: FAIL with "cannot find function `self_initialize_personal_vault`".

- [ ] **Step 7: Implement `self_initialize_personal_vault`**

Add to `kiosk-gui/src/setup.rs`. This follows the exact same
`anchor_lang::prelude::Pubkey` <-> `solana_pubkey::Pubkey` conversion
pattern as `chain.rs`'s `submit_withdraw` and `tools/admin-cli`'s
`cmd_create_vault` — both already-verified-working precedents in this
codebase:

```rust
pub fn self_initialize_personal_vault(
    rpc: &solana_client::rpc_client::RpcClient,
    owner: &solana_keypair::Keypair,
    cfg: &ProvisioningConfig,
) -> Result<(solana_pubkey::Pubkey, solana_pubkey::Pubkey), crate::chain::ChainError> {
    use anchor_lang::{InstructionData, ToAccountMetas};
    use solana_signer::Signer;

    let owner_pubkey_client = owner.pubkey();
    let program_id_client =
        solana_pubkey::Pubkey::new_from_array(cfg.program_id.to_bytes());

    let (vault, _) = solana_pubkey::Pubkey::find_program_address(
        &[b"vault", owner_pubkey_client.as_ref(), b"personal"],
        &program_id_client,
    );
    let (beneficiary_list, _) = solana_pubkey::Pubkey::find_program_address(
        &[b"beneficiaries", vault.as_ref()],
        &program_id_client,
    );

    // Idempotency by existence check, not error-string matching — see
    // this task's Interfaces note.
    if rpc.get_account_data(&vault).is_ok() {
        return Ok((vault, beneficiary_list));
    }

    let vault_token_account =
        spl_associated_token_account::get_associated_token_address(&vault, &cfg.mint);

    let to_anchor = |p: solana_pubkey::Pubkey| {
        anchor_lang::prelude::Pubkey::new_from_array(p.to_bytes())
    };

    let data = family_vault::instruction::InitializeVault {
        is_pool: false,
        daily_limit: 0,
    }
    .data();
    let accounts = family_vault::accounts::InitializeVault {
        payer: to_anchor(owner_pubkey_client),
        owner: to_anchor(owner_pubkey_client),
        vault: to_anchor(vault),
        beneficiary_list: to_anchor(beneficiary_list),
        mint: to_anchor(cfg.mint),
        vault_token_account: to_anchor(vault_token_account),
        token_program: to_anchor(spl_token::ID),
        associated_token_program: to_anchor(spl_associated_token_account::ID),
        system_program: to_anchor(solana_pubkey::Pubkey::new_from_array([0u8; 32])),
    }
    .to_account_metas(None);

    let client_metas: Vec<solana_instruction::AccountMeta> = accounts
        .iter()
        .map(|m| solana_instruction::AccountMeta {
            pubkey: solana_pubkey::Pubkey::new_from_array(m.pubkey.to_bytes()),
            is_signer: m.is_signer,
            is_writable: m.is_writable,
        })
        .collect();

    let ix = solana_instruction::Instruction {
        program_id: program_id_client,
        accounts: client_metas,
        data,
    };

    let blockhash = rpc
        .get_latest_blockhash()
        .map_err(crate::chain::map_client_error_pub)?;
    let tx = solana_transaction::Transaction::new_signed_with_payer(
        &[ix],
        Some(&owner_pubkey_client),
        &[owner],
        blockhash,
    );
    rpc.send_and_confirm_transaction(&tx)
        .map_err(crate::chain::map_client_error_pub)?;

    Ok((vault, beneficiary_list))
}
```

- [ ] **Step 8: Run test to verify it passes**

This step requires manual setup (a funded devnet keypair) — document it
rather than skip verification:

1. `solana-keygen new --no-bip39-passphrase -o /tmp/setup-test-owner.json`
2. `solana airdrop 1 /tmp/setup-test-owner.json --url https://api.devnet.solana.com` (or transfer from an already-funded address if the faucet is rate-limited — see this session's own history for that exact situation)
3. Confirm the `mint`/`program_id` values in the test match a real devnet mint/deployed program (update the test's literals if the ones already in this plan are stale by the time this task runs — devnet deploys are not permanent).
4. Run: `cargo test --package kiosk-gui setup::tests::self_init_is_idempotent -- --ignored --nocapture`

Expected: PASS, and running it a third time (manually, outside the test) should also succeed instantly (existence check short-circuits).

- [ ] **Step 9: Mark as not-yet-used, run the full non-ignored suite**

Add `#[allow(dead_code)] // used starting in Task 6` above `MIN_FUNDING_LAMPORTS_BUFFER`, `compute_min_funding_lamports`, and `self_initialize_personal_vault`.

Run: `cargo test --package kiosk-gui setup::tests -- --nocapture` (the two `#[ignore]`d tests are skipped by default, that's expected)
Expected: 7 tests pass, 2 ignored.

Run: `cargo clippy --package kiosk-gui --all-targets`
Expected: clean.

- [ ] **Step 10: Commit**

```bash
git add kiosk-gui/src/setup.rs kiosk-gui/Cargo.toml kiosk-gui/Cargo.lock
git commit -m "feat(kiosk-gui): self-initialize the personal vault once funded"
```

---

### Task 5: Setup screen (`.slint`)

**Files:**
- Create: `kiosk-gui/ui/setup.slint`
- Modify: `kiosk-gui/ui/app.slint`

**Interfaces:**
- Consumes: `Theme` (from `kiosk-gui/ui/theme.slint`, already exists — `Theme.bg`, `Theme.fg`, `Theme.spacing`, `Theme.font-size-heading`, `Theme.font-size-body`).
- Produces: `SetupScreen` component with `in property <image> qr-image`, `in property <string> status-text`; `Screen.setup` enum variant; `AppWindow`'s `in property <image> setup-qr-image <=> setup.qr-image;` and `in property <string> setup-status-text <=> setup.status-text;`.

- [ ] **Step 1: Create `kiosk-gui/ui/setup.slint`**

```slint
import { Theme } from "theme.slint";

export component SetupScreen inherits Rectangle {
    in property <image> qr-image;
    in property <string> status-text: "";

    background: Theme.bg;

    VerticalLayout {
        padding: Theme.spacing;
        spacing: Theme.spacing;
        alignment: center;

        Text {
            text: "Настройка устройства";
            color: Theme.fg;
            font-size: Theme.font-size-heading;
            horizontal-alignment: center;
        }

        Image {
            source: qr-image;
            width: 280px;
            height: 280px;
            horizontal-alignment: center;
        }

        Text {
            text: status-text;
            color: Theme.fg;
            font-size: Theme.font-size-body;
            horizontal-alignment: center;
            wrap: word-wrap;
        }
    }
}
```

No buttons, no `TouchArea`, no `accessible-role` — this screen has no
user action to take (design spec: "Никаких кнопок на этом экране —
процесс полностью автоматический").

- [ ] **Step 2: Wire `Screen.setup` and the Setup screen into `app.slint`**

In `kiosk-gui/ui/app.slint`, make these exact edits:

Add the import (alongside the other screen imports at the top):

```slint
import { SetupScreen } from "setup.slint";
```

Change the `Screen` enum to add `setup`:

```slint
export enum Screen { setup, my-wallet, amount-select, confirm, result, family-wallet }
```

Add the property bindings (alongside the other `in property` lines):

```slint
    in property <image> setup-qr-image <=> setup.qr-image;
    in property <string> setup-status-text <=> setup.status-text;
```

Add the screen instance (alongside the other screen instances, e.g. right before `wallet := MyWalletScreen`):

```slint
    setup := SetupScreen {
        width: 100%;
        height: 100%;
        visible: current-screen == Screen.setup;
    }
```

Do **not** change the enum's default: `in-out property <Screen> current-screen: Screen.my-wallet;` stays as-is — Task 6 sets it to `Screen.setup` explicitly in Rust immediately after `AppWindow::new()` when setup mode is detected, rather than changing the compiled-in default (which would put every *normal* boot briefly on the wrong screen).

- [ ] **Step 3: Run the existing test suite to confirm nothing broke**

Run: `cargo build --package kiosk-gui` (this recompiles the `.slint` files — a syntax error here would show up as a build failure, since `build.rs` compiles them)
Expected: succeeds.

Run: `cargo test --package kiosk-gui --test navigation`
Expected: 10/10 pass, unchanged (this task adds a screen nothing currently
navigates to; the existing scenarios don't touch it).

- [ ] **Step 4: Commit**

```bash
git add kiosk-gui/ui/setup.slint kiosk-gui/ui/app.slint
git commit -m "feat(kiosk-gui): add the Setup screen (QR + status text)"
```

---

### Task 6: Wire the setup flow into `main()`

**Files:**
- Modify: `kiosk-gui/src/main.rs`

**Interfaces:**
- Consumes: everything from Tasks 1-5 (`setup::needs_setup`, `setup::owner_keypair_path`, `setup::load_provisioning_config`, `setup::generate_or_load_owner_keypair`, `setup::render_qr`, `setup::compute_min_funding_lamports`, `setup::self_initialize_personal_vault`, `setup::write_kiosk_config`, `setup::KioskConfigValues`, `Screen::Setup`, `AppWindow::set_setup_qr_image`/`set_setup_status_text`).
- Produces: nothing further downstream — last task in this plan.

This task is integration work (real network calls driving real UI state
across a retry loop) — not meaningfully unit-testable beyond what Tasks
1-4 already cover in isolation. Its verification is `cargo check`/`cargo
test --test navigation` (regression guard) plus the manual devnet
procedure in Step 4.

- [ ] **Step 1: Replace the top of `main()` in `kiosk-gui/src/main.rs`**

Currently `main()` starts with:

```rust
fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;

    app.set_presets(
```

Change it to check for setup mode first, and branch:

```rust
fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;
    let config_path = config_path();

    if setup::needs_setup(&config_path) {
        run_setup_flow(app.as_weak(), config_path);
        return app.run();
    }

    app.set_presets(
```

The rest of `main()` (from `app.set_presets(...)` through `app.run()` at
the very end) stays exactly as it is today — untouched by this task.

- [ ] **Step 2: Add `run_setup_flow`**

Add this function near the other top-level functions in
`kiosk-gui/src/main.rs` (e.g. right above `fn fetch_beneficiary_addresses`):

```rust
/// Drives the entire first-run setup flow on a background thread: load
/// provisioning config, generate-or-load the owner keypair, show its QR,
/// poll for funding, self-initialize the vault, write kiosk.toml, then
/// switch to the normal My Wallet flow — all without restarting the
/// process. See the design spec's "Фандинг и самостоятельная
/// инициализация" section for the exact sequence this implements.
fn run_setup_flow(app_weak: slint::Weak<AppWindow>, config_path: std::path::PathBuf) {
    let app_weak_for_screen = app_weak.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(app) = app_weak_for_screen.upgrade() {
            app.set_current_screen(Screen::Setup);
        }
    });

    std::thread::spawn(move || {
        let config_dir = config_path
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let provisioning_path = config_dir.join("provisioning.toml");

        let provisioning = match setup::load_provisioning_config(&provisioning_path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[kiosk-gui] setup: failed to load provisioning.toml: {e}");
                set_setup_status(
                    &app_weak,
                    "Не получилось. Обратитесь в поддержку".to_string(),
                );
                return;
            }
        };

        let keypair_path = setup::owner_keypair_path(&config_dir);
        let owner = match setup::generate_or_load_owner_keypair(&keypair_path) {
            Ok(k) => k,
            Err(e) => {
                eprintln!("[kiosk-gui] setup: failed to generate/load keypair: {e}");
                set_setup_status(
                    &app_weak,
                    "Не получилось. Обратитесь в поддержку".to_string(),
                );
                return;
            }
        };

        let owner_pubkey_str = solana_signer::Signer::pubkey(&owner).to_string();
        let qr = setup::render_qr(&owner_pubkey_str);
        {
            let app_weak = app_weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(app) = app_weak.upgrade() {
                    app.set_setup_qr_image(qr);
                    app.set_setup_status_text(
                        "Покажите этот код родственнику, у которого будет доступ к приложению семьи"
                            .into(),
                    );
                }
            });
        }

        let rpc = solana_client::rpc_client::RpcClient::new(provisioning.rpc_url.clone());
        let owner_pubkey_client = solana_signer::Signer::pubkey(&owner);

        let min_lamports = loop {
            match setup::compute_min_funding_lamports(&rpc) {
                Ok(v) => break v,
                Err(_) => std::thread::sleep(std::time::Duration::from_secs(4)),
            }
        };

        loop {
            match rpc.get_balance(&owner_pubkey_client) {
                Ok(balance) if balance >= min_lamports => break,
                _ => std::thread::sleep(std::time::Duration::from_secs(4)),
            }
        }

        set_setup_status(&app_weak, "Почти готово...".to_string());

        let mut vault_result = None;
        for attempt in 0..5u32 {
            match setup::self_initialize_personal_vault(&rpc, &owner, &provisioning) {
                Ok(v) => {
                    vault_result = Some(v);
                    break;
                }
                Err(e) => {
                    eprintln!(
                        "[kiosk-gui] setup: initialize_vault attempt {attempt} failed: {e:?}"
                    );
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            }
        }

        let Some((vault, beneficiary_list)) = vault_result else {
            set_setup_status(
                &app_weak,
                "Не получилось. Обратитесь в поддержку".to_string(),
            );
            return;
        };

        let values = setup::KioskConfigValues {
            rpc_url: provisioning.rpc_url.clone(),
            keypair_path: keypair_path.to_string_lossy().into_owned(),
            program_id: provisioning.program_id.to_string(),
            mint: provisioning.mint.to_string(),
            personal_vault: vault.to_string(),
            personal_beneficiary_list: beneficiary_list.to_string(),
        };
        if let Err(e) = setup::write_kiosk_config(&config_path, &values) {
            eprintln!("[kiosk-gui] setup: failed to write kiosk.toml: {e}");
            set_setup_status(
                &app_weak,
                "Не получилось. Обратитесь в поддержку".to_string(),
            );
            return;
        }

        // Load the config we just wrote (exercises the exact same code
        // path a normal boot uses) and switch straight into the normal
        // flow in this same process — no restart.
        let cfg = match chain::load_config(&config_path) {
            Ok(cfg) => std::sync::Arc::new(cfg),
            Err(e) => {
                eprintln!("[kiosk-gui] setup: wrote kiosk.toml but failed to reload it: {e}");
                set_setup_status(
                    &app_weak,
                    "Не получилось. Обратитесь в поддержку".to_string(),
                );
                return;
            }
        };

        let personal_beneficiaries: std::sync::Arc<
            std::sync::Mutex<Vec<(u8, solana_pubkey::Pubkey)>>,
        > = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let pool_beneficiaries: std::sync::Arc<
            std::sync::Mutex<Vec<(u8, solana_pubkey::Pubkey)>>,
        > = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

        let app_weak_for_transition = app_weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(app) = app_weak_for_transition.upgrade() {
                app.set_presets(
                    std::rc::Rc::new(slint::VecModel::from(vec![
                        "10 USDC".into(),
                        "20 USDC".into(),
                        "50 USDC".into(),
                    ]))
                    .into(),
                );
                app.set_current_screen(Screen::MyWallet);
            }
        });
        refresh_vault_state(app_weak, cfg, personal_beneficiaries, pool_beneficiaries);
    });
}

fn set_setup_status(app_weak: &slint::Weak<AppWindow>, text: String) {
    let app_weak = app_weak.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(app) = app_weak.upgrade() {
            app.set_setup_status_text(text.into());
        }
    });
}
```

Note: after `run_setup_flow` transitions to `Screen::MyWallet`, none of
the callback wiring further down in `main()` (`on_beneficiary_selected`,
`on_send_confirmed`, etc.) has been attached yet in the setup-mode
branch, because `main()` `return`s early via `app.run()` right after
calling `run_setup_flow` (see Step 1). This is intentional and correct
for the **first ever boot**: the freshly-created `kiosk.toml` now exists
on disk, but the callbacks were never wired for *this* process
instance. Tapping a beneficiary button on the newly-shown My Wallet
screen would silently do nothing.

Fix this by **not** returning early — instead, after starting the
setup flow, fall through to the same callback-wiring code that already
exists for the normal path, using a `cfg` that starts as `None` and
gets populated once setup finishes. Revise Step 1 and the immediately
following code once more:

- [ ] **Step 1 (revised): thread an `Option<Arc<ChainConfig>>` through setup and normal startup**

Replace the plan's Step 1 change with this instead (supersedes it — do
not apply Step 1's change verbatim, apply this one):

```rust
fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;
    let config_path = config_path();

    let cfg: Arc<Mutex<Option<Arc<ChainConfig>>>> = Arc::new(Mutex::new(None));

    if setup::needs_setup(&config_path) {
        let cfg_slot = cfg.clone();
        run_setup_flow(app.as_weak(), config_path.clone(), cfg_slot);
    } else {
        match chain::load_config(&config_path) {
            Ok(loaded) => {
                *cfg.lock().unwrap() = Some(Arc::new(loaded));
            }
            Err(e) => {
                eprintln!("[kiosk-gui] failed to load config: {e}");
                app.set_balance_text("Нет связи. Попробуй ещё раз позже".into());
                app.set_family_balance_text("Нет связи. Попробуй ещё раз позже".into());
                app.run()?;
                return Ok(());
            }
        }
    }

    app.set_presets(
        Rc::new(slint::VecModel::from(vec![
            "10 USDC".into(),
            "20 USDC".into(),
            "50 USDC".into(),
        ]))
        .into(),
    );
```

Delete the old `let cfg = match chain::load_config(&config_path) { ... };`
block that used to follow `app.set_presets(...)` — it's now handled
above, before `set_presets`, for both branches.

Every later place in `main()` that does `cfg.clone()` (there are several
— in the startup-fetch call, the refresh timer, `on_send_confirmed`,
`on_family_beneficiary_selected`, `on_ok_pressed`) now needs to go
through the `Option`: wrap each such use so it's a no-op when `cfg` is
still `None` (setup not finished yet). Concretely, change each call site
from something like:

```rust
    refresh_vault_state(
        app.as_weak(),
        cfg.clone(),
        personal_beneficiaries.clone(),
        pool_beneficiaries.clone(),
    );
```

to a small helper used everywhere instead of calling `refresh_vault_state`
directly:

```rust
fn refresh_vault_state_if_ready(
    app_weak: slint::Weak<AppWindow>,
    cfg: &Arc<Mutex<Option<Arc<ChainConfig>>>>,
    personal_beneficiaries: Arc<Mutex<Vec<(u8, solana_pubkey::Pubkey)>>>,
    pool_beneficiaries: Arc<Mutex<Vec<(u8, solana_pubkey::Pubkey)>>>,
) {
    let Some(cfg) = cfg.lock().unwrap().clone() else {
        return;
    };
    refresh_vault_state(app_weak, cfg, personal_beneficiaries, pool_beneficiaries);
}
```

Replace every existing `refresh_vault_state(...)` call site in `main()`
(the initial-fetch call, the timer closure, and the `on_ok_pressed`
closure — 3 call sites total, all already present from the previous
plan) with `refresh_vault_state_if_ready(...)`, passing `&cfg` instead
of `cfg.clone()`, and keep every other argument the same.

For `on_send_confirmed` and `on_family_beneficiary_selected`, which read
`cfg` to call `chain::submit_withdraw`, change:

```rust
        let cfg = cfg.clone();
```

(captured before `std::thread::spawn`) to:

```rust
        let Some(cfg) = cfg.lock().unwrap().clone() else {
            app.set_result_success(false);
            app.set_result_message_text(
                "Не получилось. Попробуй ещё раз или позови родственника".into(),
            );
            app.set_current_screen(Screen::Result);
            return;
        };
```

placed at the top of each closure body (this can only actually trigger
if a user somehow reaches My Wallet before setup finished, which
shouldn't happen given Setup screen has no navigation away from it —
this is defense in depth, not an expected path).

Every closure in `main()` that currently does `let cfg = cfg.clone();`
at the top (to capture it for a `move` closure) now needs to capture
`cfg.clone()` of the `Arc<Mutex<Option<Arc<ChainConfig>>>>` itself
(cheap, just an `Arc` clone) instead of the old `Arc<ChainConfig>`
clone — the actual unwrap-or-return happens inside the closure body at
call time, as shown above, not at closure-creation time.

- [ ] **Step 2: Update `run_setup_flow`'s signature and final step**

Change `run_setup_flow`'s signature to accept the shared slot:

```rust
fn run_setup_flow(
    app_weak: slint::Weak<AppWindow>,
    config_path: std::path::PathBuf,
    cfg_slot: Arc<Mutex<Option<Arc<ChainConfig>>>>,
) {
```

And change its final section (the part that used to call
`refresh_vault_state` directly at the very end) to populate the slot
first:

```rust
        *cfg_slot.lock().unwrap() = Some(cfg.clone());

        let personal_beneficiaries: std::sync::Arc<
            std::sync::Mutex<Vec<(u8, solana_pubkey::Pubkey)>>,
        > = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let pool_beneficiaries: std::sync::Arc<
            std::sync::Mutex<Vec<(u8, solana_pubkey::Pubkey)>>,
        > = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

        let app_weak_for_transition = app_weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(app) = app_weak_for_transition.upgrade() {
                app.set_current_screen(Screen::MyWallet);
            }
        });
        refresh_vault_state(app_weak, cfg, personal_beneficiaries, pool_beneficiaries);
```

(This replaces the equivalent block from the plan's first draft of
`run_setup_flow` above — same logic, now writing into `cfg_slot` instead
of assuming it's the only source of `cfg`. Also remove the
`app.set_presets(...)` call that was inside this closure in the first
draft — `set_presets` now happens once in `main()` itself, per Step 1
(revised), not duplicated here.)

- [ ] **Step 3: Run `cargo check`, fix any remaining `cfg.clone()` call sites**

Run: `cargo check --package kiosk-gui`

This step exists because the exact list of `cfg.clone()` call sites in
the current `main.rs` may have shifted slightly by the time this task
runs (earlier tasks in this plan don't touch `main.rs`, but review fixes
from prior plans might have). Fix every compile error by applying the
same transformation shown in Step 1 (revised) — wrap each read of `cfg`
in a `.lock().unwrap().clone()` + `let Some(cfg) = ... else { ... }`
pattern, never a bare `Arc<ChainConfig>` clone of the old kind.

Expected once fixed: compiles cleanly, no `dead_code`/`unsafe_code`
denials (crate has `[lints.rust] unsafe_code = "deny", dead_code =
"deny"`).

- [ ] **Step 4: Run the regression suite**

Run: `cargo test --package kiosk-gui --test navigation`
Expected: 10/10 pass — these tests construct `AppWindow` directly and
never call `main()`, so this task's changes can't affect them; this is
purely a regression guard.

Run: `cargo test --package kiosk-gui setup::tests -- --nocapture`
Expected: 7 pass, 2 ignored (unchanged from Task 4).

Run: `cargo clippy --workspace --all-targets`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add kiosk-gui/src/main.rs
git commit -m "feat(kiosk-gui): wire the first-run setup flow into main()"
```

- [ ] **Step 6: Document the manual devnet verification (do not automate)**

No code change — the design spec calls for a real end-to-end check, and
this plan's earlier tasks already established the pattern (real devnet
smoke tests, documented not scripted) for exactly this kind of
verification. Perform once, report the outcome in this task's final
report:

1. Pick (or deploy fresh) a devnet `family-vault` program id and a
   devnet SPL mint (this session's history already has working examples
   of both — `solana program deploy`, `spl-token create-token`).
2. Write `/tmp/setup-test/provisioning.toml`:
   ```toml
   rpc_url = "https://api.devnet.solana.com"
   program_id = "<the program id from step 1>"
   mint = "<the mint from step 1>"
   ```
3. Confirm `/tmp/setup-test/kiosk.toml` and
   `/tmp/setup-test/owner-keypair.json` do **not** exist.
4. Run: `cargo run --package kiosk-gui -- --config /tmp/setup-test/kiosk.toml`

   (Note: `config_path()`'s current logic resolves the provisioning file
   from `config_path.parent()` — passing `--config
   /tmp/setup-test/kiosk.toml` makes it look for
   `/tmp/setup-test/provisioning.toml`, matching step 2.)
5. Verify: the Setup screen appears with a QR code and the "Покажите
   этот код..." text.
6. Decode the QR (any phone QR scanner) — confirm it's a valid base58
   Solana pubkey.
7. `solana transfer <that pubkey> 0.01 --allow-unfunded-recipient --url
   https://api.devnet.solana.com` from a funded devnet keypair (adjust
   the amount if `compute_min_funding_lamports`'s real value at test
   time is higher — check the eprintln'd attempt logs, or just send a
   generous 0.02-0.05 SOL to avoid the exact-threshold edge case).
8. Verify: within a few seconds of the transfer landing, the screen
   switches to My Wallet showing a real (likely "0 USDC", since nothing
   was deposited) balance, with no beneficiaries yet.
9. Verify: `/tmp/setup-test/kiosk.toml` now exists and its
   `personal_vault`/`personal_beneficiary_list` match what
   `solana account <owner pubkey>`-derived PDAs would predict, and
   `pool_vault`/`pool_beneficiary_list` equal `personal_vault`/
   `personal_beneficiary_list` (the placeholder).
10. Kill and restart the same command (`cargo run --package kiosk-gui --
    --config /tmp/setup-test/kiosk.toml`) — verify it now boots straight
    into the normal flow (no Setup screen), confirming `needs_setup`
    correctly reads the now-existing `kiosk.toml`.

---

## Self-review notes (from plan authoring)

- **Spec coverage:** `provisioning.toml` (Task 1), keypair generate-or-load
  with resumability (Task 1), QR rendering (Task 2), `kiosk.toml` writing
  with the pool-placeholder pattern (Task 3), funding poll + idempotent
  self-init with the 5-attempt cap (Task 4/6), Setup screen with no
  buttons/no animation (Task 5), full wiring with no-process-restart
  transition (Task 6) — every section of the design spec has a task.
- **Type consistency:** `ProvisioningConfig`, `ProvisioningError`,
  `KioskConfigValues`, `needs_setup`, `owner_keypair_path`,
  `load_provisioning_config`, `generate_or_load_owner_keypair`,
  `render_qr`, `compute_min_funding_lamports`,
  `self_initialize_personal_vault`, `write_kiosk_config` — each defined
  exactly once (Tasks 1-4), used with matching names/signatures in
  Task 6.
- **A structural note surfaced only while writing Task 6, not caught
  during brainstorming:** the original single-`Arc<ChainConfig>`
  capture-at-closure-creation pattern (used throughout the existing
  `main.rs` from the blockchain-wiring plan) assumed `cfg` is always
  known at the moment every closure is created. Setup mode breaks that
  assumption — `cfg` doesn't exist yet when the closures are wired. Task
  6 threads an `Arc<Mutex<Option<Arc<ChainConfig>>>>` through instead and
  resolves it at call time in every closure body. This is the one place
  this plan revises its own first draft mid-task (visible above as
  "Step 1 (revised)") — left in rather than silently smoothed over, so
  the implementer sees why the shape differs from a naive reading of
  Task 6's Interfaces line.
