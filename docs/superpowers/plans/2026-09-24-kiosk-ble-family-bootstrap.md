# Kiosk BLE Family-Member Bootstrap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** During the kiosk's first-run setup, after its personal vault exists, let one family member hand the kiosk their pubkey and name over Bluetooth LE — authenticated by scanning a QR code the kiosk shows (with a typed 6-digit fallback code) — and register them on-chain as that vault's first trusted approver and payee.

**Architecture:** A shared wire-format module in `family-vault-client` (used later by family-app too), kiosk-only credential/session logic (`kiosk-gui/src/pairing.rs`), the on-chain side (`kiosk-gui/src/family_member.rs`: idempotent, one atomic transaction), and a thin BLE peripheral adapter (`kiosk-gui/src/ble.rs`, `bluer`). `run_setup_flow` gains one step between vault initialisation and writing `kiosk.toml`, so it resumes cleanly after power loss. BLE-unavailable or skipped means setup simply continues.

**Tech Stack:** Rust, `bluer 0.17.4` (BlueZ bindings, peripheral role), `tokio` (current-thread runtime inside the setup thread), Slint UI, existing `family-vault` Anchor program, existing `family-vault-client` crate, existing `setup::render_qr`.

**Spec:** `docs/superpowers/specs/2026-09-23-family-app-design.md` — sections "Identity & Bootstrap" (mechanism, fallback code, wire format, failure behaviour). Related: `docs/superpowers/specs/2026-09-23-kiosk-first-run-setup-design.md`.

## Global Constraints

- Work directly on `master` (established pattern for this repo). End every commit message with the line `Claude-Session: https://claude.ai/code/session_01VLZAgeertuGTkdt8LXpiEJ`.
- `kiosk-gui` and `family-vault-client` both carry `[lints.rust] unsafe_code = "deny", dead_code = "deny"`. In the `kiosk-gui` binary, modules not yet consumed get a module-level `#[allow(dead_code)] // used starting in Task 6` on their `mod` line in `main.rs`; **Task 6 removes all three** and must then compile clean (that proves nothing was left unused).
- Two incompatible Solana `Pubkey` generations coexist: `anchor_lang::prelude::Pubkey` (Anchor bindings, `ProvisioningConfig.program_id`) and `solana_pubkey::Pubkey` (client side). Convert ONLY with `family_vault_client::{to_anchor, to_client}`; never hand-roll `new_from_array(x.to_bytes())` in new code.
- Exact dependency pins, empirically verified to compile together in `kiosk-gui` with `solana-client =4.2.2` and no duplicate `tokio`: `bluer = { version = "0.17.4", features = ["bluetoothd"] }`, `tokio = { version = "1", features = ["rt", "time", "macros", "sync"] }`, `futures = "0.3"`. Do not change versions or features.
- `kiosk-gui` becomes Linux-only (`bluer` is BlueZ-only). Accepted: the kiosk is Linux hardware.
- The UI thread never does I/O. Background work talks to Slint only via `slint::invoke_from_event_loop`. `slint::Image` is not `Send`: build it (`setup::render_qr`) inside the UI-thread closure from a `String`.
- **Never log the secret or the 6-digit code.** Logging the pubkey and name is fine.
- UI copy is plain Russian, no jargon (no "BLE", "GATT", "pubkey", "vault" on screen). Use `Theme` tokens only, 60px minimum touch targets, no animation or blinking (project WCAG 2.2 AAA / ETSI EG 203 350 conventions).
- Tests use real logic, not mocks. Hardware-dependent tests are `#[ignore = "..."]` and run manually.

## File Structure

- Create `tools/family-vault-client/src/pairing.rs` — shared wire format: constants, `AuthField`, `PairingRequest`, `FrameError`, `auth_from_pin`, `encode_qr_payload`/`decode_qr_payload`, `encode_frame`, `FrameAssembler`.
- Modify `tools/family-vault-client/src/lib.rs` (`pub mod pairing;`, `to_client_metas`), `tools/family-vault-client/Cargo.toml`.
- Create `kiosk-gui/src/pairing.rs` — kiosk-side `PairingCredentials`, `AuthGate`, `PairingSession`.
- Create `kiosk-gui/src/family_member.rs` — membership planning, instruction building, RPC glue.
- Create `kiosk-gui/src/ble.rs` — `bluer` peripheral adapter.
- Modify `kiosk-gui/src/setup.rs` (funding floor), `kiosk-gui/src/chain.rs` (use `to_client_metas`), `kiosk-gui/src/main.rs`, `kiosk-gui/Cargo.toml`.
- Modify `kiosk-gui/ui/theme.slint`, `kiosk-gui/ui/setup.slint`, `kiosk-gui/ui/app.slint`, `kiosk-gui/tests/navigation.rs`.

---

### Task 1: Shared pairing wire format in `family-vault-client`

**Files:**
- Create: `tools/family-vault-client/src/pairing.rs`
- Modify: `tools/family-vault-client/src/lib.rs` (add `pub mod pairing;` under the crate docs)
- Modify: `tools/family-vault-client/Cargo.toml` (add dev-dependency)

**Interfaces:**
- Consumes: `solana_pubkey::Pubkey`.
- Produces (all `pub`, module `family_vault_client::pairing`):
  - `SERVICE_UUID_U128: u128`, `PAIRING_CHARACTERISTIC_UUID_U128: u128`
  - `AUTH_LEN: usize = 16`, `PIN_LEN: usize = 6`, `PUBKEY_LEN: usize = 32`, `MAX_NAME_LEN: usize = 32`, `STALE_AFTER: Duration`, `QR_PREFIX: &str = "fvpair1:"`
  - `type AuthField = [u8; AUTH_LEN]`
  - `struct PairingRequest { auth: AuthField, pubkey: Pubkey, name: String }` (`Debug, Clone, PartialEq, Eq`)
  - `enum FrameError { BadPin, NameEmpty, NameTooLong, NameNotUtf8, TooLong }` (`Debug, Clone, Copy, PartialEq, Eq`)
  - `fn auth_from_pin(&str) -> Result<AuthField, FrameError>` (6 ASCII digits, zero-padded to 16 bytes)
  - `fn encode_qr_payload(&AuthField) -> String` (`"fvpair1:" + 32 lowercase hex chars`), `fn decode_qr_payload(&str) -> Option<AuthField>`
  - `fn encode_frame(&PairingRequest) -> Result<Vec<u8>, FrameError>`
  - `struct FrameAssembler` (`Debug, Default`) with `fn push(&mut self, chunk: &[u8], now: std::time::Instant) -> Result<Option<PairingRequest>, FrameError>`

- [ ] **Step 1: Add the dev-dependency and register the module**

In `tools/family-vault-client/Cargo.toml`, append:

```toml

[dev-dependencies]
family-vault = { version = "0.1.0", path = "../../programs/family-vault", features = ["cpi"] }
```

In `tools/family-vault-client/src/lib.rs`, add directly below the crate-level `//!` doc block (before the first `use`):

```rust
pub mod pairing;
```

- [ ] **Step 2: Write the failing tests**

Create `tools/family-vault-client/src/pairing.rs` containing ONLY this test module (the implementation comes in Step 4):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn sample() -> PairingRequest {
        PairingRequest {
            auth: [0xAB; AUTH_LEN],
            pubkey: Pubkey::new_from_array([7u8; 32]),
            name: "Внук Саша".to_string(),
        }
    }

    fn valid_frame() -> Vec<u8> {
        encode_frame(&sample()).unwrap()
    }

    // --- frame assembly -------------------------------------------------

    #[test]
    fn a_frame_delivered_in_one_write_round_trips() {
        let mut assembler = FrameAssembler::default();
        let out = assembler.push(&valid_frame(), Instant::now()).unwrap();
        assert_eq!(out, Some(sample()));
    }

    #[test]
    fn a_frame_split_into_20_byte_writes_round_trips() {
        let mut assembler = FrameAssembler::default();
        let now = Instant::now();
        let frame = valid_frame();
        let mut chunks = frame.chunks(20).peekable();
        while let Some(chunk) = chunks.next() {
            let out = assembler.push(chunk, now).unwrap();
            if chunks.peek().is_some() {
                assert_eq!(out, None, "must ask for more until the last chunk");
            } else {
                assert_eq!(out, Some(sample()));
            }
        }
    }

    #[test]
    fn a_frame_delivered_one_byte_at_a_time_round_trips() {
        let mut assembler = FrameAssembler::default();
        let now = Instant::now();
        let frame = valid_frame();
        let last = frame.len() - 1;
        for (i, byte) in frame.iter().enumerate() {
            let out = assembler.push(&[*byte], now).unwrap();
            if i == last {
                assert_eq!(out, Some(sample()));
            } else {
                assert_eq!(out, None);
            }
        }
    }

    #[test]
    fn a_stale_partial_frame_is_discarded_before_the_next_write() {
        let mut assembler = FrameAssembler::default();
        let t0 = Instant::now();
        let frame = valid_frame();
        assert_eq!(assembler.push(&frame[..10], t0).unwrap(), None);
        // Without stale-discarding, these 10 leftover bytes plus a full
        // frame would overshoot and error out as TooLong.
        let later = t0 + STALE_AFTER + Duration::from_secs(1);
        assert_eq!(assembler.push(&frame, later).unwrap(), Some(sample()));
    }

    #[test]
    fn a_fresh_partial_frame_is_kept() {
        let mut assembler = FrameAssembler::default();
        let t0 = Instant::now();
        let frame = valid_frame();
        assert_eq!(assembler.push(&frame[..10], t0).unwrap(), None);
        let soon = t0 + Duration::from_secs(1);
        assert_eq!(assembler.push(&frame[10..], soon).unwrap(), Some(sample()));
    }

    fn header_with_name_len(n: u8) -> Vec<u8> {
        let mut frame = Vec::new();
        frame.extend_from_slice(&[0xAB; AUTH_LEN]);
        frame.extend_from_slice(&[7u8; 32]);
        frame.push(n);
        frame
    }

    #[test]
    fn a_zero_length_name_is_rejected() {
        let mut assembler = FrameAssembler::default();
        assert_eq!(
            assembler.push(&header_with_name_len(0), Instant::now()),
            Err(FrameError::NameEmpty)
        );
    }

    #[test]
    fn a_name_longer_than_32_bytes_is_rejected_as_soon_as_the_header_is_complete() {
        let mut assembler = FrameAssembler::default();
        assert_eq!(
            assembler.push(&header_with_name_len(33), Instant::now()),
            Err(FrameError::NameTooLong)
        );
    }

    #[test]
    fn trailing_bytes_after_a_complete_frame_are_rejected() {
        let mut frame = valid_frame();
        frame.push(0);
        let mut assembler = FrameAssembler::default();
        assert_eq!(
            assembler.push(&frame, Instant::now()),
            Err(FrameError::TooLong)
        );
    }

    #[test]
    fn a_name_that_is_not_utf8_is_rejected() {
        let mut frame = header_with_name_len(2);
        frame.extend_from_slice(&[0xff, 0xfe]);
        let mut assembler = FrameAssembler::default();
        assert_eq!(
            assembler.push(&frame, Instant::now()),
            Err(FrameError::NameNotUtf8)
        );
    }

    #[test]
    fn a_bad_frame_does_not_poison_the_next_one() {
        let mut assembler = FrameAssembler::default();
        let now = Instant::now();
        let mut bad = valid_frame();
        bad.push(0);
        assert_eq!(assembler.push(&bad, now), Err(FrameError::TooLong));
        assert_eq!(assembler.push(&valid_frame(), now).unwrap(), Some(sample()));
    }

    #[test]
    fn encode_rejects_bad_names() {
        let mut req = sample();
        req.name = String::new();
        assert_eq!(encode_frame(&req), Err(FrameError::NameEmpty));

        let mut req = sample();
        req.name = "a".repeat(33);
        assert_eq!(encode_frame(&req), Err(FrameError::NameTooLong));
    }

    // --- fallback code as an auth field ---------------------------------

    #[test]
    fn auth_from_pin_is_the_six_digits_zero_padded() {
        let field = auth_from_pin("482913").unwrap();
        assert_eq!(&field[..6], b"482913");
        assert!(field[6..].iter().all(|b| *b == 0));
    }

    #[test]
    fn auth_from_pin_rejects_anything_but_six_ascii_digits() {
        assert_eq!(auth_from_pin("12345"), Err(FrameError::BadPin));
        assert_eq!(auth_from_pin("1234567"), Err(FrameError::BadPin));
        assert_eq!(auth_from_pin("12345x"), Err(FrameError::BadPin));
        assert_eq!(auth_from_pin("١٢٣٤٥٦"), Err(FrameError::BadPin));
        assert_eq!(auth_from_pin(""), Err(FrameError::BadPin));
    }

    // --- QR payload -----------------------------------------------------

    #[test]
    fn a_qr_payload_round_trips() {
        let secret: AuthField = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd,
            0xfe, 0xff,
        ];
        let text = encode_qr_payload(&secret);
        assert_eq!(text, "fvpair1:0001020304050607f8f9fafbfcfdfeff");
        assert_eq!(decode_qr_payload(&text), Some(secret));
    }

    #[test]
    fn decode_accepts_uppercase_hex() {
        let secret: AuthField = [0xAB; AUTH_LEN];
        let upper = format!("fvpair1:{}", "AB".repeat(16));
        assert_eq!(decode_qr_payload(&upper), Some(secret));
    }

    #[test]
    fn decode_rejects_malformed_payloads() {
        let good = format!("fvpair1:{}", "ab".repeat(16));
        assert!(decode_qr_payload(&good).is_some());

        assert_eq!(decode_qr_payload(&"ab".repeat(16)), None, "missing prefix");
        assert_eq!(
            decode_qr_payload(&format!("fvpair2:{}", "ab".repeat(16))),
            None,
            "unknown version"
        );
        assert_eq!(decode_qr_payload("fvpair1:abcd"), None, "too short");
        assert_eq!(
            decode_qr_payload(&format!("fvpair1:{}", "ab".repeat(17))),
            None,
            "too long"
        );
        assert_eq!(
            decode_qr_payload(&format!("fvpair1:{}", "g".repeat(32))),
            None,
            "not hex"
        );
        // `u8::from_str_radix` alone would accept a leading '+'.
        assert_eq!(
            decode_qr_payload(&format!("fvpair1:{}", "+f".repeat(16))),
            None,
            "sign characters are not hex digits"
        );
    }

    // --- misc -----------------------------------------------------------

    #[test]
    fn max_name_len_matches_the_on_chain_limit() {
        assert_eq!(MAX_NAME_LEN, family_vault::state::BENEFICIARY_NAME_LEN);
    }

    // Manual helper for testing the kiosk with a phone BLE tool (e.g. nRF
    // Connect): prints the hex of a frame to paste into a write. Give it
    // EITHER the text scanned from the kiosk's QR OR the fallback code:
    //   PAIR_QR='fvpair1:…' PAIR_PUBKEY=<base58> PAIR_NAME="Внук Саша" \
    //   cargo test -p family-vault-client print_frame_hex -- --ignored --nocapture
    //   PAIR_PIN=123456 PAIR_PUBKEY=<base58> PAIR_NAME="Внук Саша" \
    //   cargo test -p family-vault-client print_frame_hex -- --ignored --nocapture
    #[test]
    #[ignore = "manual helper: reads PAIR_QR or PAIR_PIN, plus PAIR_PUBKEY and PAIR_NAME"]
    fn print_frame_hex() {
        let auth = match std::env::var("PAIR_QR") {
            Ok(text) => decode_qr_payload(&text).expect("PAIR_QR is not a valid pairing QR text"),
            Err(_) => auth_from_pin(&std::env::var("PAIR_PIN").expect("set PAIR_QR or PAIR_PIN"))
                .expect("PAIR_PIN must be six digits"),
        };
        let request = PairingRequest {
            auth,
            pubkey: std::env::var("PAIR_PUBKEY")
                .expect("set PAIR_PUBKEY")
                .parse()
                .expect("PAIR_PUBKEY must be a valid base58 pubkey"),
            name: std::env::var("PAIR_NAME").expect("set PAIR_NAME"),
        };
        let frame = encode_frame(&request).expect("valid input");
        let hex: String = frame.iter().map(|b| format!("{b:02X}")).collect();
        println!("{hex}");
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test --package family-vault-client pairing::tests 2>&1 | tail -20`
Expected: compile FAIL — unresolved names `PairingRequest`, `encode_frame`, `FrameAssembler`, `FrameError`, `AUTH_LEN`, `STALE_AFTER`, `MAX_NAME_LEN`, `auth_from_pin`, `encode_qr_payload`, `decode_qr_payload`, `AuthField`, `Pubkey`.

- [ ] **Step 4: Write the implementation**

Insert this at the TOP of `tools/family-vault-client/src/pairing.rs`, above the `#[cfg(test)]` block:

```rust
//! Wire format for first-run family-member pairing over BLE, shared by the
//! kiosk (BLE peripheral) and family-app (BLE central) so both sides use
//! the same encoder/decoder. See the family-app design spec, "Identity &
//! Bootstrap".
//!
//! One writable characteristic carries a single frame:
//!
//! ```text
//! [ 16 bytes: auth field ]
//! [ 32 bytes: the family member's pubkey ]
//! [ 1 byte: N, the name length in bytes, 1..=32 ]
//! [ N bytes: UTF-8 name ]
//! ```
//!
//! The auth field is either the 16-byte secret carried by the QR code the
//! kiosk shows ([`encode_qr_payload`]) or the kiosk's 6-digit fallback code
//! zero-padded to 16 bytes ([`auth_from_pin`]); the kiosk accepts either.
//!
//! A central may deliver the frame in one write or several (an ATT payload
//! can be as small as 20 bytes), so the receiving side reassembles with
//! [`FrameAssembler`].

use std::time::{Duration, Instant};

use solana_pubkey::Pubkey;

/// BLE GATT identifiers as raw `u128`, so this crate depends on no BLE
/// stack: the kiosk converts with `bluer::Uuid::from_u128`, family-app with
/// `uuid::Uuid::from_u128`.
pub const SERVICE_UUID_U128: u128 = 0x8f1a0000_5b1e_4c6a_9a7e_0d2f3a1b4c5d;
pub const PAIRING_CHARACTERISTIC_UUID_U128: u128 = 0x8f1a0001_5b1e_4c6a_9a7e_0d2f3a1b4c5d;

pub const AUTH_LEN: usize = 16;
pub const PIN_LEN: usize = 6;
pub const PUBKEY_LEN: usize = 32;
/// Mirrors `family_vault::state::BENEFICIARY_NAME_LEN` (a test pins them
/// together) — the program rejects longer names.
pub const MAX_NAME_LEN: usize = 32;
const HEADER_LEN: usize = AUTH_LEN + PUBKEY_LEN + 1;

/// Prefix (and version) of the text encoded in the kiosk's pairing QR.
pub const QR_PREFIX: &str = "fvpair1:";

/// A partially-received frame older than this is dropped before the next
/// write is appended, so a central that gave up halfway cannot corrupt the
/// next attempt.
pub const STALE_AFTER: Duration = Duration::from_secs(3);

pub type AuthField = [u8; AUTH_LEN];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingRequest {
    pub auth: AuthField,
    pub pubkey: Pubkey,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// The fallback code is not exactly six ASCII digits.
    BadPin,
    NameEmpty,
    NameTooLong,
    NameNotUtf8,
    /// More bytes arrived than the declared name length allows.
    TooLong,
}

/// The fallback code as an auth field: the six ASCII digits, zero-padded.
pub fn auth_from_pin(pin: &str) -> Result<AuthField, FrameError> {
    if pin.len() != PIN_LEN || !pin.bytes().all(|b| b.is_ascii_digit()) {
        return Err(FrameError::BadPin);
    }
    let mut field = [0u8; AUTH_LEN];
    field[..PIN_LEN].copy_from_slice(pin.as_bytes());
    Ok(field)
}

/// The text the kiosk encodes in its pairing QR: [`QR_PREFIX`] followed by
/// the secret as 32 lowercase hex characters.
pub fn encode_qr_payload(secret: &AuthField) -> String {
    let hex: String = secret.iter().map(|b| format!("{b:02x}")).collect();
    format!("{QR_PREFIX}{hex}")
}

/// Parses text scanned from the kiosk's QR. Strict: exact prefix, exactly
/// 32 hex digits (either case).
pub fn decode_qr_payload(text: &str) -> Option<AuthField> {
    let hex = text.strip_prefix(QR_PREFIX)?;
    if hex.len() != AUTH_LEN * 2 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut secret = [0u8; AUTH_LEN];
    for (i, byte) in secret.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(secret)
}

pub fn encode_frame(request: &PairingRequest) -> Result<Vec<u8>, FrameError> {
    let name = request.name.as_bytes();
    if name.is_empty() {
        return Err(FrameError::NameEmpty);
    }
    if name.len() > MAX_NAME_LEN {
        return Err(FrameError::NameTooLong);
    }
    let mut frame = Vec::with_capacity(HEADER_LEN + name.len());
    frame.extend_from_slice(&request.auth);
    frame.extend_from_slice(&request.pubkey.to_bytes());
    frame.push(name.len() as u8);
    frame.extend_from_slice(name);
    Ok(frame)
}

/// Reassembles a frame from however many writes the central used.
#[derive(Debug, Default)]
pub struct FrameAssembler {
    buf: Vec<u8>,
    last_write: Option<Instant>,
}

impl FrameAssembler {
    /// Feeds one write. `Ok(Some(_))` means a complete, valid frame was
    /// assembled; `Ok(None)` means more bytes are needed; `Err(_)` means
    /// the frame is malformed. After `Ok(Some(_))` or `Err(_)` the buffer
    /// is cleared, so the next write starts a fresh frame. The auth field
    /// is opaque here — checking it is the receiver's job.
    pub fn push(
        &mut self,
        chunk: &[u8],
        now: Instant,
    ) -> Result<Option<PairingRequest>, FrameError> {
        if let Some(last) = self.last_write {
            if now.saturating_duration_since(last) > STALE_AFTER {
                self.buf.clear();
            }
        }
        self.last_write = Some(now);
        self.buf.extend_from_slice(chunk);

        let parsed = self.try_parse();
        if !matches!(parsed, Ok(None)) {
            self.buf.clear();
        }
        parsed
    }

    fn try_parse(&self) -> Result<Option<PairingRequest>, FrameError> {
        if self.buf.len() < HEADER_LEN {
            return Ok(None);
        }
        let name_len = self.buf[HEADER_LEN - 1] as usize;
        if name_len == 0 {
            return Err(FrameError::NameEmpty);
        }
        if name_len > MAX_NAME_LEN {
            return Err(FrameError::NameTooLong);
        }
        let total = HEADER_LEN + name_len;
        if self.buf.len() < total {
            return Ok(None);
        }
        if self.buf.len() > total {
            return Err(FrameError::TooLong);
        }

        let mut auth = [0u8; AUTH_LEN];
        auth.copy_from_slice(&self.buf[..AUTH_LEN]);

        let mut pubkey_bytes = [0u8; PUBKEY_LEN];
        pubkey_bytes.copy_from_slice(&self.buf[AUTH_LEN..AUTH_LEN + PUBKEY_LEN]);

        let name = std::str::from_utf8(&self.buf[HEADER_LEN..total])
            .map_err(|_| FrameError::NameNotUtf8)?
            .to_string();

        Ok(Some(PairingRequest {
            auth,
            pubkey: Pubkey::new_from_array(pubkey_bytes),
            name,
        }))
    }
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --package family-vault-client 2>&1 | tail -30`
Expected: PASS — the 9 existing tests plus 17 new pairing tests (`26 passed; 0 failed; 1 ignored`).

- [ ] **Step 6: Lint and commit**

Run: `cargo clippy --package family-vault-client --all-targets 2>&1 | tail -10` — expected: clean.

```bash
git add tools/family-vault-client
git commit -m "feat(family-vault-client): shared BLE pairing wire format

Frame = auth(16) + pubkey(32) + name length(1) + name. The auth field is
the secret from the kiosk's QR or the zero-padded 6-digit fallback code.
Shared so the kiosk (peripheral) and family-app (central) use one
encoder/decoder. Assembler tolerates any write chunking and discards stale
partial frames; QR payload parsing is strict.

Claude-Session: https://claude.ai/code/session_01VLZAgeertuGTkdt8LXpiEJ"
```

---

### Task 2: Kiosk-side credentials, gate and `PairingSession`

**Files:**
- Create: `kiosk-gui/src/pairing.rs`
- Modify: `kiosk-gui/src/main.rs` (declare the module)

**Interfaces:**
- Consumes (Task 1): `family_vault_client::pairing::{AuthField, AUTH_LEN, PIN_LEN, FrameAssembler, FrameError, PairingRequest, encode_qr_payload, auth_from_pin, encode_frame}`.
- Produces (module `crate::pairing`):
  - `MAX_WRONG_ATTEMPTS: u32 = 5`
  - `struct PairingCredentials` (`Debug, Clone, PartialEq, Eq`): `from_random_bytes([u8; 20]) -> Self`, `secret(&self) -> &AuthField`, `pin(&self) -> &str`, `qr_payload(&self) -> String`
  - `fn generate_credentials() -> std::io::Result<PairingCredentials>` (OS RNG)
  - `fn display_pin(pin: &str) -> String` (`"482913"` → `"482 913"`)
  - `enum AuthCheck { Accepted, Wrong { attempts_left: u32 }, LockedOut }`, `struct AuthGate` (`new(PairingCredentials)`, `check(&mut self, &AuthField) -> AuthCheck`, `rotate(&mut self, PairingCredentials)`)
  - `enum WriteOutcome { NeedMore, Accepted(PairingRequest), WrongAuth { attempts_left: u32 }, LockedOut { new_credentials: Option<PairingCredentials> }, Malformed(FrameError) }` (`Debug, PartialEq, Eq`)
  - `struct PairingSession` (`new(PairingCredentials)`, `on_write(&mut self, chunk: &[u8], now: Instant, next_credentials: impl FnOnce() -> Option<PairingCredentials>) -> WriteOutcome`)

- [ ] **Step 1: Declare the module**

In `kiosk-gui/src/main.rs`, directly after the existing `mod chain;` / `mod setup;` lines, add:

```rust
#[allow(dead_code)] // used starting in Task 6
mod pairing;
```

- [ ] **Step 2: Write the failing tests**

Create `kiosk-gui/src/pairing.rs` containing ONLY this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use family_vault_client::pairing::{auth_from_pin, encode_frame, PairingRequest};
    use solana_pubkey::Pubkey;

    /// secret = [1; 16]; pin = u32::from_le_bytes([1;4]) % 1_000_000 = 843009
    fn creds() -> PairingCredentials {
        PairingCredentials::from_random_bytes([1u8; 20])
    }

    fn other_creds() -> PairingCredentials {
        PairingCredentials::from_random_bytes([2u8; 20])
    }

    fn frame_with_auth(auth: AuthField) -> Vec<u8> {
        encode_frame(&request_with_auth(auth)).unwrap()
    }

    fn request_with_auth(auth: AuthField) -> PairingRequest {
        PairingRequest {
            auth,
            pubkey: Pubkey::new_from_array([7u8; 32]),
            name: "Внук Саша".to_string(),
        }
    }

    fn wrong_auth() -> AuthField {
        [0xEE; AUTH_LEN]
    }

    // --- credentials ----------------------------------------------------

    #[test]
    fn from_random_bytes_derives_a_zero_padded_six_digit_pin() {
        let mut bytes = [9u8; 20];
        bytes[16..].copy_from_slice(&42u32.to_le_bytes());
        let c = PairingCredentials::from_random_bytes(bytes);
        assert_eq!(c.pin(), "000042");
        assert_eq!(c.secret(), &[9u8; 16]);
    }

    #[test]
    fn the_pin_field_matches_the_shared_encoding_of_the_pin() {
        let c = creds();
        assert_eq!(c.pin_field, auth_from_pin(c.pin()).unwrap());
    }

    #[test]
    fn qr_payload_carries_the_secret() {
        let c = creds();
        assert_eq!(
            family_vault_client::pairing::decode_qr_payload(&c.qr_payload()),
            Some(*c.secret())
        );
    }

    #[test]
    fn generated_credentials_are_well_formed_and_differ_between_calls() {
        let a = generate_credentials().unwrap();
        let b = generate_credentials().unwrap();
        assert_eq!(a.pin().len(), 6);
        assert!(a.pin().bytes().all(|x| x.is_ascii_digit()));
        assert_ne!(a.secret(), b.secret());
    }

    #[test]
    fn display_pin_groups_digits_in_threes() {
        assert_eq!(display_pin("482913"), "482 913");
    }

    // --- gate -----------------------------------------------------------

    #[test]
    fn gate_accepts_the_qr_secret() {
        let mut gate = AuthGate::new(creds());
        assert_eq!(gate.check(creds().secret()), AuthCheck::Accepted);
    }

    #[test]
    fn gate_accepts_the_fallback_code() {
        let mut gate = AuthGate::new(creds());
        let field = auth_from_pin(creds().pin()).unwrap();
        assert_eq!(gate.check(&field), AuthCheck::Accepted);
    }

    #[test]
    fn gate_rejects_an_all_zero_auth_field() {
        // Guards against ever treating an unset/zero field as a match.
        let mut gate = AuthGate::new(creds());
        assert_eq!(
            gate.check(&[0u8; AUTH_LEN]),
            AuthCheck::Wrong { attempts_left: 4 }
        );
    }

    #[test]
    fn gate_counts_down_wrong_attempts_then_locks_out() {
        let mut gate = AuthGate::new(creds());
        assert_eq!(gate.check(&wrong_auth()), AuthCheck::Wrong { attempts_left: 4 });
        assert_eq!(gate.check(&wrong_auth()), AuthCheck::Wrong { attempts_left: 3 });
        assert_eq!(gate.check(&wrong_auth()), AuthCheck::Wrong { attempts_left: 2 });
        assert_eq!(gate.check(&wrong_auth()), AuthCheck::Wrong { attempts_left: 1 });
        assert_eq!(gate.check(&wrong_auth()), AuthCheck::LockedOut);
    }

    #[test]
    fn a_locked_gate_rejects_even_correct_credentials_until_rotated() {
        let mut gate = AuthGate::new(creds());
        for _ in 0..MAX_WRONG_ATTEMPTS {
            gate.check(&wrong_auth());
        }
        assert_eq!(gate.check(creds().secret()), AuthCheck::LockedOut);

        gate.rotate(other_creds());
        // Old credentials are dead, new ones work.
        assert_eq!(
            gate.check(creds().secret()),
            AuthCheck::Wrong { attempts_left: 4 }
        );
        assert_eq!(
            gate.check(&auth_from_pin(creds().pin()).unwrap()),
            AuthCheck::Wrong { attempts_left: 3 }
        );
        assert_eq!(gate.check(other_creds().secret()), AuthCheck::Accepted);
    }

    // --- session --------------------------------------------------------

    #[test]
    fn session_accepts_the_qr_secret_in_one_write() {
        let mut session = PairingSession::new(creds());
        let out = session.on_write(&frame_with_auth(*creds().secret()), Instant::now(), || None);
        assert_eq!(
            out,
            WriteOutcome::Accepted(request_with_auth(*creds().secret()))
        );
    }

    #[test]
    fn session_accepts_the_fallback_code() {
        let mut session = PairingSession::new(creds());
        let field = auth_from_pin(creds().pin()).unwrap();
        let out = session.on_write(&frame_with_auth(field), Instant::now(), || None);
        assert_eq!(out, WriteOutcome::Accepted(request_with_auth(field)));
    }

    #[test]
    fn session_asks_for_more_until_the_frame_is_complete() {
        let mut session = PairingSession::new(creds());
        let now = Instant::now();
        let frame = frame_with_auth(*creds().secret());
        assert_eq!(session.on_write(&frame[..20], now, || None), WriteOutcome::NeedMore);
        assert_eq!(
            session.on_write(&frame[20..], now, || None),
            WriteOutcome::Accepted(request_with_auth(*creds().secret()))
        );
    }

    #[test]
    fn session_reports_wrong_credentials() {
        let mut session = PairingSession::new(creds());
        let out = session.on_write(&frame_with_auth(wrong_auth()), Instant::now(), || None);
        assert_eq!(out, WriteOutcome::WrongAuth { attempts_left: 4 });
    }

    #[test]
    fn session_reports_malformed_frames() {
        let mut session = PairingSession::new(creds());
        let mut frame = frame_with_auth(*creds().secret());
        frame.push(0); // trailing byte
        let out = session.on_write(&frame, Instant::now(), || None);
        assert_eq!(out, WriteOutcome::Malformed(FrameError::TooLong));
    }

    #[test]
    fn session_rotates_credentials_after_too_many_wrong_attempts() {
        let mut session = PairingSession::new(creds());
        let now = Instant::now();
        for _ in 0..4 {
            session.on_write(&frame_with_auth(wrong_auth()), now, || None);
        }
        let out = session.on_write(&frame_with_auth(wrong_auth()), now, || Some(other_creds()));
        assert_eq!(
            out,
            WriteOutcome::LockedOut {
                new_credentials: Some(other_creds())
            }
        );

        // The old QR secret and old code are dead, the new secret works.
        assert_eq!(
            session.on_write(&frame_with_auth(*creds().secret()), now, || None),
            WriteOutcome::WrongAuth { attempts_left: 4 }
        );
        assert_eq!(
            session.on_write(&frame_with_auth(*other_creds().secret()), now, || None),
            WriteOutcome::Accepted(request_with_auth(*other_creds().secret()))
        );
    }

    #[test]
    fn session_stays_locked_if_new_credentials_cannot_be_generated() {
        let mut session = PairingSession::new(creds());
        let now = Instant::now();
        for _ in 0..5 {
            session.on_write(&frame_with_auth(wrong_auth()), now, || None);
        }
        // Even the correct secret must not get in while locked and un-rotated.
        let out = session.on_write(&frame_with_auth(*creds().secret()), now, || None);
        assert_eq!(out, WriteOutcome::LockedOut { new_credentials: None });
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test --package kiosk-gui pairing::tests 2>&1 | tail -15`
Expected: compile FAIL — unresolved `PairingCredentials`, `generate_credentials`, `display_pin`, `AuthGate`, `AuthCheck`, `PairingSession`, `WriteOutcome`, `MAX_WRONG_ATTEMPTS`, `AuthField`, `AUTH_LEN`, `Instant`, `FrameError`.

- [ ] **Step 4: Write the implementation**

Insert at the TOP of `kiosk-gui/src/pairing.rs`, above the test module:

```rust
//! Kiosk-side logic for first-run family-member pairing: the credentials
//! the kiosk shows (a QR carrying a random secret, plus a 6-digit fallback
//! code), the gate that checks them, and `PairingSession`, which turns raw
//! BLE writes into an outcome. Pure logic — no Bluetooth and no chain I/O
//! here (see `ble.rs` and `family_member.rs`), so it is fully unit-tested.
//! The wire format itself lives in `family_vault_client::pairing`.

use std::time::Instant;

use family_vault_client::pairing::{
    encode_qr_payload, AuthField, FrameAssembler, FrameError, PairingRequest, AUTH_LEN, PIN_LEN,
};

/// After this many wrong attempts the kiosk rotates to fresh credentials,
/// so the guessable 6-digit code cannot be brute-forced.
pub const MAX_WRONG_ATTEMPTS: u32 = 5;

/// What the kiosk currently accepts: the 128-bit secret shown as a QR, and
/// the 6-digit fallback code shown under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingCredentials {
    secret: AuthField,
    pin: String,
    /// `pin` zero-padded to 16 bytes, exactly as a central sends it.
    pin_field: AuthField,
}

impl PairingCredentials {
    /// 16 bytes of secret followed by 4 bytes that become the PIN. (The
    /// modulo bias of `u32 % 1_000_000` is ~0.02% and irrelevant for a
    /// rotating code.)
    pub fn from_random_bytes(bytes: [u8; 20]) -> Self {
        let mut secret = [0u8; AUTH_LEN];
        secret.copy_from_slice(&bytes[..AUTH_LEN]);
        let number = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) % 1_000_000;
        let pin = format!("{number:06}");
        // `{:06}` of a value < 1_000_000 is always exactly six ASCII
        // digits, so this cannot go out of bounds.
        let mut pin_field = [0u8; AUTH_LEN];
        pin_field[..PIN_LEN].copy_from_slice(pin.as_bytes());
        Self {
            secret,
            pin,
            pin_field,
        }
    }

    pub fn secret(&self) -> &AuthField {
        &self.secret
    }

    pub fn pin(&self) -> &str {
        &self.pin
    }

    /// The text to render as the pairing QR.
    pub fn qr_payload(&self) -> String {
        encode_qr_payload(&self.secret)
    }
}

/// Fresh credentials from the OS RNG.
pub fn generate_credentials() -> std::io::Result<PairingCredentials> {
    use std::io::Read;
    let mut bytes = [0u8; 20];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(PairingCredentials::from_random_bytes(bytes))
}

/// `"482913"` -> `"482 913"`, easier to read off a screen and type.
pub fn display_pin(pin: &str) -> String {
    let (head, tail) = pin.split_at(pin.len().min(3));
    format!("{head} {tail}")
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[derive(Debug, PartialEq, Eq)]
pub enum AuthCheck {
    Accepted,
    Wrong { attempts_left: u32 },
    LockedOut,
}

pub struct AuthGate {
    credentials: PairingCredentials,
    wrong_attempts: u32,
}

impl AuthGate {
    pub fn new(credentials: PairingCredentials) -> Self {
        Self {
            credentials,
            wrong_attempts: 0,
        }
    }

    /// Accepts the QR secret OR the fallback code. Once locked out, every
    /// candidate (even a correct one) is refused until `rotate` installs new
    /// credentials.
    pub fn check(&mut self, auth: &AuthField) -> AuthCheck {
        if self.wrong_attempts >= MAX_WRONG_ATTEMPTS {
            return AuthCheck::LockedOut;
        }
        // Evaluate both comparisons (no short-circuit) to keep the timing
        // independent of which credential matched.
        let by_secret = constant_time_eq(auth, &self.credentials.secret);
        let by_pin = constant_time_eq(auth, &self.credentials.pin_field);
        if by_secret | by_pin {
            self.wrong_attempts = 0;
            return AuthCheck::Accepted;
        }
        self.wrong_attempts += 1;
        if self.wrong_attempts >= MAX_WRONG_ATTEMPTS {
            AuthCheck::LockedOut
        } else {
            AuthCheck::Wrong {
                attempts_left: MAX_WRONG_ATTEMPTS - self.wrong_attempts,
            }
        }
    }

    pub fn rotate(&mut self, credentials: PairingCredentials) {
        self.credentials = credentials;
        self.wrong_attempts = 0;
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum WriteOutcome {
    /// A partial frame was buffered; wait for more writes.
    NeedMore,
    /// A complete frame carrying valid credentials.
    Accepted(PairingRequest),
    WrongAuth { attempts_left: u32 },
    /// Too many wrong attempts. `new_credentials` are the freshly generated
    /// ones to show, or `None` if generating them failed (the gate then
    /// stays locked, which is the safe direction).
    LockedOut {
        new_credentials: Option<PairingCredentials>,
    },
    Malformed(FrameError),
}

pub struct PairingSession {
    assembler: FrameAssembler,
    gate: AuthGate,
}

impl PairingSession {
    pub fn new(credentials: PairingCredentials) -> Self {
        Self {
            assembler: FrameAssembler::default(),
            gate: AuthGate::new(credentials),
        }
    }

    /// Handles one BLE write. `next_credentials` is only called when the
    /// gate locks out, to produce the replacement credentials.
    pub fn on_write(
        &mut self,
        chunk: &[u8],
        now: Instant,
        next_credentials: impl FnOnce() -> Option<PairingCredentials>,
    ) -> WriteOutcome {
        match self.assembler.push(chunk, now) {
            Ok(None) => WriteOutcome::NeedMore,
            Err(e) => WriteOutcome::Malformed(e),
            Ok(Some(request)) => match self.gate.check(&request.auth) {
                AuthCheck::Accepted => WriteOutcome::Accepted(request),
                AuthCheck::Wrong { attempts_left } => WriteOutcome::WrongAuth { attempts_left },
                AuthCheck::LockedOut => {
                    let new_credentials = next_credentials();
                    if let Some(credentials) = &new_credentials {
                        self.gate.rotate(credentials.clone());
                    }
                    WriteOutcome::LockedOut { new_credentials }
                }
            },
        }
    }
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --package kiosk-gui pairing::tests 2>&1 | tail -25`
Expected: PASS — `18 passed; 0 failed`.

- [ ] **Step 6: Lint and commit**

Run: `cargo clippy --package kiosk-gui --all-targets 2>&1 | tail -10` — expected: clean (the module-level `allow(dead_code)` covers the not-yet-consumed items).

```bash
git add kiosk-gui/src/pairing.rs kiosk-gui/src/main.rs
git commit -m "feat(kiosk-gui): pairing credentials, gate and session

Credentials are a 128-bit secret (for the QR) plus a 6-digit fallback
code, both from the OS RNG. The gate accepts either, compares in constant
time, locks out after 5 wrong attempts and rotates (staying locked if
rotation fails). PairingSession turns raw BLE writes into one outcome.

Claude-Session: https://claude.ai/code/session_01VLZAgeertuGTkdt8LXpiEJ"
```

---

### Task 3: On-chain side — idempotent family-member registration

**Files:**
- Create: `kiosk-gui/src/family_member.rs`
- Modify: `tools/family-vault-client/src/lib.rs`, `tools/family-vault-client/Cargo.toml` (`to_client_metas`)
- Modify: `kiosk-gui/src/chain.rs`, `kiosk-gui/src/setup.rs` (use `to_client_metas`; grow the funding floor)
- Modify: `kiosk-gui/src/main.rs` (declare the module)

**Interfaces:**
- Consumes: `family_vault_client::{approvers_pda, beneficiary_list_pda, to_anchor, AnchorPubkey}`, `crate::chain::{map_client_error_pub, ChainError}`, `family_vault::{instruction, accounts, state::{TrustedApprovers, BeneficiaryList, MAX_APPROVERS, MAX_BENEFICIARIES, Beneficiary}}`.
- Produces:
  - `family_vault_client::to_client_metas(&[anchor_lang::prelude::AccountMeta]) -> Vec<solana_instruction::AccountMeta>`
  - `crate::setup::min_funding_from_rents(vault_rent: u64, list_rent: u64, approvers_rent: u64, ata_rent: u64) -> u64`
  - module `crate::family_member`: `MemberSteps { init_approvers, add_approver, add_beneficiary: bool }` (+ `is_empty()`), `MembershipState { approvers: Option<TrustedApprovers>, beneficiaries: BeneficiaryList }`, `MemberContext { program_id, owner, vault: solana_pubkey::Pubkey }`, `plan_member_steps(Option<&TrustedApprovers>, &BeneficiaryList, &Pubkey) -> MemberSteps`, `needs_pairing(&MembershipState) -> bool`, `build_member_instructions(&MemberContext, &MemberSteps, family: &Pubkey, name: &str) -> Vec<Instruction>`, `fetch_membership_state(&RpcClient, approvers: &Pubkey, beneficiary_list: &Pubkey) -> Result<MembershipState, ChainError>`, `add_first_family_member(&RpcClient, owner: &Keypair, program_id: &Pubkey, vault: &Pubkey, family: &Pubkey, name: &str) -> Result<(), ChainError>`

- [ ] **Step 1: Failing test for `to_client_metas`**

In `tools/family-vault-client/Cargo.toml` add to `[dependencies]`: `solana-instruction = "3.0.0"`.

Append to the `tests` module in `tools/family-vault-client/src/lib.rs`:

```rust
    #[test]
    fn to_client_metas_preserves_pubkey_and_flags() {
        let key = pubkey(5);
        let metas = vec![
            anchor_lang::prelude::AccountMeta::new(to_anchor(key), true),
            anchor_lang::prelude::AccountMeta::new_readonly(to_anchor(pubkey(6)), false),
        ];
        let converted = to_client_metas(&metas);
        assert_eq!(converted.len(), 2);
        assert_eq!(converted[0].pubkey, key);
        assert!(converted[0].is_signer && converted[0].is_writable);
        assert_eq!(converted[1].pubkey, pubkey(6));
        assert!(!converted[1].is_signer && !converted[1].is_writable);
    }
```

Run: `cargo test --package family-vault-client to_client_metas 2>&1 | tail -8`
Expected: compile FAIL — `to_client_metas` not found.

- [ ] **Step 2: Implement `to_client_metas` and migrate the two existing copies**

In `tools/family-vault-client/src/lib.rs`, add below `to_client`:

```rust
/// Converts Anchor-generated `AccountMeta`s (old Solana crate generation)
/// into the client-generation `AccountMeta`s that `solana_instruction::
/// Instruction` needs. Every place that builds an instruction from an
/// Anchor `accounts` struct needs exactly this conversion.
pub fn to_client_metas(
    metas: &[anchor_lang::prelude::AccountMeta],
) -> Vec<solana_instruction::AccountMeta> {
    metas
        .iter()
        .map(|m| solana_instruction::AccountMeta {
            pubkey: to_client(m.pubkey),
            is_signer: m.is_signer,
            is_writable: m.is_writable,
        })
        .collect()
}
```

In `kiosk-gui/src/chain.rs` (`submit_withdraw`), replace the whole `let client_metas: Vec<solana_instruction::AccountMeta> = anchor_metas ... .collect();` statement with:

```rust
    let client_metas = family_vault_client::to_client_metas(&anchor_metas);
```

In `kiosk-gui/src/setup.rs` (`self_initialize_personal_vault`), replace the same-shaped statement (it maps over `accounts`) with:

```rust
    let client_metas = family_vault_client::to_client_metas(&accounts);
```

Run: `cargo test --package family-vault-client 2>&1 | tail -6 && cargo check --package kiosk-gui 2>&1 | tail -5`
Expected: tests PASS, kiosk-gui compiles clean. (If `anchor_lang::prelude::AccountMeta` does not resolve, use `anchor_lang::solana_program::instruction::AccountMeta` — Anchor's prelude re-exports that same type.)

- [ ] **Step 3: Failing test — the funding floor must include the approvers account**

In `kiosk-gui/src/setup.rs`, add to its `tests` module:

```rust
    #[test]
    fn min_funding_includes_every_account_rent_plus_the_buffer() {
        assert_eq!(
            min_funding_from_rents(1_000, 2_000, 400, 30_000),
            1_000 + 2_000 + 400 + 30_000 + MIN_FUNDING_LAMPORTS_BUFFER
        );
        // The approvers account is created during family pairing, so its
        // rent must be part of the floor: dropping it changes the total.
        assert_ne!(
            min_funding_from_rents(1_000, 2_000, 400, 30_000),
            min_funding_from_rents(1_000, 2_000, 0, 30_000)
        );
    }
```

Run: `cargo test --package kiosk-gui setup::tests::min_funding 2>&1 | tail -6` — expected: compile FAIL, `min_funding_from_rents` not found.

- [ ] **Step 4: Implement the funding floor change**

In `kiosk-gui/src/setup.rs`, replace the body of `compute_min_funding_lamports` (keep its signature and any existing doc comment, and extend the doc comment with: "Includes the `TrustedApprovers` account created during first-run family pairing.") with:

```rust
    let vault_rent = rpc
        .get_minimum_balance_for_rent_exemption(family_vault::state::Vault::SPACE)
        .map_err(crate::chain::map_client_error_pub)?;
    let list_rent = rpc
        .get_minimum_balance_for_rent_exemption(family_vault::state::BeneficiaryList::SPACE)
        .map_err(crate::chain::map_client_error_pub)?;
    let approvers_rent = rpc
        .get_minimum_balance_for_rent_exemption(family_vault::state::TrustedApprovers::SPACE)
        .map_err(crate::chain::map_client_error_pub)?;
    let ata_rent = rpc
        .get_minimum_balance_for_rent_exemption(165) // spl_token::state::Account::LEN
        .map_err(crate::chain::map_client_error_pub)?;
    Ok(min_funding_from_rents(
        vault_rent,
        list_rent,
        approvers_rent,
        ata_rent,
    ))
```

and add directly below that function:

```rust
/// Pure so the composition of the floor is unit-tested without an RPC.
pub fn min_funding_from_rents(
    vault_rent: u64,
    list_rent: u64,
    approvers_rent: u64,
    ata_rent: u64,
) -> u64 {
    vault_rent + list_rent + approvers_rent + ata_rent + MIN_FUNDING_LAMPORTS_BUFFER
}
```

Run: `cargo test --package kiosk-gui setup::tests 2>&1 | tail -6` — expected: `8 passed; 0 failed; 2 ignored`.

- [ ] **Step 5: Declare the module and write the failing tests**

In `kiosk-gui/src/main.rs`, next to the other `mod` lines add:

```rust
#[allow(dead_code)] // used starting in Task 6
mod family_member;
```

Create `kiosk-gui/src/family_member.rs` containing ONLY this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::InstructionData;
    use family_vault::state::{Beneficiary, MAX_APPROVERS, MAX_BENEFICIARIES};
    use family_vault_client::AnchorPubkey;

    fn key(byte: u8) -> Pubkey {
        Pubkey::new_from_array([byte; 32])
    }

    fn approvers_with(members: &[Pubkey]) -> TrustedApprovers {
        let mut entries = [AnchorPubkey::default(); MAX_APPROVERS];
        for (i, m) in members.iter().enumerate() {
            entries[i] = to_anchor(*m);
        }
        TrustedApprovers {
            vault: Default::default(),
            count: members.len() as u8,
            entries,
        }
    }

    fn list_with(members: &[Pubkey]) -> BeneficiaryList {
        let mut entries = [Beneficiary::default(); MAX_BENEFICIARIES];
        for (i, m) in members.iter().enumerate() {
            entries[i].address = to_anchor(*m);
            entries[i].name_len = 1;
        }
        BeneficiaryList {
            vault: Default::default(),
            count: members.len() as u8,
            entries,
        }
    }

    fn ctx() -> MemberContext {
        MemberContext {
            program_id: key(1),
            owner: key(2),
            vault: key(3),
        }
    }

    #[test]
    fn plans_everything_when_nothing_exists_yet() {
        let steps = plan_member_steps(None, &list_with(&[]), &key(9));
        assert_eq!(
            steps,
            MemberSteps {
                init_approvers: true,
                add_approver: true,
                add_beneficiary: true
            }
        );
    }

    #[test]
    fn skips_init_when_the_approvers_account_exists() {
        let approvers = approvers_with(&[]);
        let steps = plan_member_steps(Some(&approvers), &list_with(&[]), &key(9));
        assert_eq!(
            steps,
            MemberSteps {
                init_approvers: false,
                add_approver: true,
                add_beneficiary: true
            }
        );
    }

    #[test]
    fn skips_whatever_is_already_done() {
        let approvers = approvers_with(&[key(9)]);
        let steps = plan_member_steps(Some(&approvers), &list_with(&[]), &key(9));
        assert_eq!(
            steps,
            MemberSteps {
                init_approvers: false,
                add_approver: false,
                add_beneficiary: true
            }
        );

        let approvers = approvers_with(&[key(9)]);
        let steps = plan_member_steps(Some(&approvers), &list_with(&[key(9)]), &key(9));
        assert!(steps.is_empty());
    }

    #[test]
    fn a_different_existing_member_does_not_count_as_done() {
        let approvers = approvers_with(&[key(4)]);
        let steps = plan_member_steps(Some(&approvers), &list_with(&[key(4)]), &key(9));
        assert!(steps.add_approver && steps.add_beneficiary && !steps.init_approvers);
    }

    #[test]
    fn pairing_is_needed_until_someone_is_an_approver() {
        let none = MembershipState {
            approvers: None,
            beneficiaries: list_with(&[]),
        };
        assert!(needs_pairing(&none));

        let empty = MembershipState {
            approvers: Some(approvers_with(&[])),
            beneficiaries: list_with(&[]),
        };
        assert!(needs_pairing(&empty));

        let done = MembershipState {
            approvers: Some(approvers_with(&[key(9)])),
            beneficiaries: list_with(&[key(9)]),
        };
        assert!(!needs_pairing(&done));
    }

    #[test]
    fn builds_all_three_instructions_in_dependency_order() {
        let family = key(9);
        let steps = MemberSteps {
            init_approvers: true,
            add_approver: true,
            add_beneficiary: true,
        };
        let ixs = build_member_instructions(&ctx(), &steps, &family, "Внук Саша");

        assert_eq!(ixs.len(), 3);
        assert_eq!(
            ixs[0].data,
            family_vault::instruction::InitializeApprovers {}.data()
        );
        assert_eq!(
            ixs[1].data,
            family_vault::instruction::AddTrustedApprover {
                address: to_anchor(family)
            }
            .data()
        );
        assert_eq!(
            ixs[2].data,
            family_vault::instruction::AddBeneficiary {
                name: "Внук Саша".to_string(),
                address: to_anchor(family)
            }
            .data()
        );
        for ix in &ixs {
            assert_eq!(ix.program_id, ctx().program_id);
        }
    }

    #[test]
    fn the_owner_signs_every_instruction() {
        let steps = MemberSteps {
            init_approvers: true,
            add_approver: true,
            add_beneficiary: true,
        };
        let ixs = build_member_instructions(&ctx(), &steps, &key(9), "Внук Саша");
        for ix in &ixs {
            assert!(
                ix.accounts
                    .iter()
                    .any(|m| m.pubkey == ctx().owner && m.is_signer),
                "owner must be a signer"
            );
        }
    }

    #[test]
    fn builds_only_the_needed_instructions() {
        let steps = MemberSteps {
            init_approvers: false,
            add_approver: false,
            add_beneficiary: true,
        };
        let ixs = build_member_instructions(&ctx(), &steps, &key(9), "Внук Саша");
        assert_eq!(ixs.len(), 1);
        assert_eq!(
            ixs[0].data,
            family_vault::instruction::AddBeneficiary {
                name: "Внук Саша".to_string(),
                address: to_anchor(key(9))
            }
            .data()
        );

        let none = MemberSteps {
            init_approvers: false,
            add_approver: false,
            add_beneficiary: false,
        };
        assert!(build_member_instructions(&ctx(), &none, &key(9), "x").is_empty());
    }
}
```

- [ ] **Step 6: Run to verify failure**

Run: `cargo test --package kiosk-gui family_member::tests 2>&1 | tail -10`
Expected: compile FAIL — `MemberSteps`, `plan_member_steps`, `needs_pairing`, `build_member_instructions`, etc. not found.

- [ ] **Step 7: Write the implementation**

Insert at the TOP of `kiosk-gui/src/family_member.rs`:

```rust
//! Chain side of first-run family-member pairing: registers the family
//! member the kiosk just met over Bluetooth as this vault's first trusted
//! approver AND payee, signed by the device's own owner key.
//!
//! Idempotent by construction: it reads the on-chain state first and only
//! submits what is missing, all in ONE transaction (atomic — either every
//! missing step lands or none does), so a crash/retry mid-setup can never
//! double-add or leave a half-registered member.

use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use family_vault::state::{BeneficiaryList, TrustedApprovers};
use family_vault_client::{approvers_pda, beneficiary_list_pda, to_anchor, to_client_metas};
use solana_client::rpc_client::RpcClient;
use solana_instruction::Instruction;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;

use crate::chain::{map_client_error_pub, ChainError};

#[derive(Debug, PartialEq, Eq)]
pub struct MemberSteps {
    pub init_approvers: bool,
    pub add_approver: bool,
    pub add_beneficiary: bool,
}

impl MemberSteps {
    pub fn is_empty(&self) -> bool {
        !self.init_approvers && !self.add_approver && !self.add_beneficiary
    }
}

pub struct MembershipState {
    /// `None` when the approvers account has not been created yet.
    pub approvers: Option<TrustedApprovers>,
    pub beneficiaries: BeneficiaryList,
}

pub struct MemberContext {
    pub program_id: Pubkey,
    pub owner: Pubkey,
    pub vault: Pubkey,
}

/// Pure: decides which of the three instructions are still needed.
pub fn plan_member_steps(
    approvers: Option<&TrustedApprovers>,
    beneficiaries: &BeneficiaryList,
    family: &Pubkey,
) -> MemberSteps {
    let family = to_anchor(*family);
    let add_approver = match approvers {
        Some(a) => !a.entries.iter().take(a.count as usize).any(|e| *e == family),
        None => true,
    };
    let add_beneficiary = !beneficiaries
        .entries
        .iter()
        .take(beneficiaries.count as usize)
        .any(|b| b.address == family);
    MemberSteps {
        init_approvers: approvers.is_none(),
        add_approver,
        add_beneficiary,
    }
}

/// Pairing is skipped on resume once someone is already an approver (the
/// transaction from a previous, interrupted setup attempt landed).
pub fn needs_pairing(state: &MembershipState) -> bool {
    match &state.approvers {
        None => true,
        Some(a) => a.count == 0,
    }
}

/// Builds the needed instructions in dependency order:
/// init approvers -> add approver -> add beneficiary.
pub fn build_member_instructions(
    ctx: &MemberContext,
    steps: &MemberSteps,
    family: &Pubkey,
    name: &str,
) -> Vec<Instruction> {
    let (approvers, _) = approvers_pda(&ctx.program_id, &ctx.vault);
    let (beneficiary_list, _) = beneficiary_list_pda(&ctx.program_id, &ctx.vault);
    let system_program = Pubkey::new_from_array([0u8; 32]);
    let mut ixs = Vec::new();

    if steps.init_approvers {
        let accounts = family_vault::accounts::InitializeApprovers {
            payer: to_anchor(ctx.owner),
            vault: to_anchor(ctx.vault),
            approvers: to_anchor(approvers),
            system_program: to_anchor(system_program),
        }
        .to_account_metas(None);
        ixs.push(Instruction {
            program_id: ctx.program_id,
            accounts: to_client_metas(&accounts),
            data: family_vault::instruction::InitializeApprovers {}.data(),
        });
    }

    if steps.add_approver {
        let accounts = family_vault::accounts::AddTrustedApprover {
            signer: to_anchor(ctx.owner),
            vault: to_anchor(ctx.vault),
            approvers: to_anchor(approvers),
            existing_approver: None,
        }
        .to_account_metas(None);
        ixs.push(Instruction {
            program_id: ctx.program_id,
            accounts: to_client_metas(&accounts),
            data: family_vault::instruction::AddTrustedApprover {
                address: to_anchor(*family),
            }
            .data(),
        });
    }

    if steps.add_beneficiary {
        let accounts = family_vault::accounts::AddBeneficiary {
            signer: to_anchor(ctx.owner),
            vault: to_anchor(ctx.vault),
            beneficiary_list: to_anchor(beneficiary_list),
        }
        .to_account_metas(None);
        ixs.push(Instruction {
            program_id: ctx.program_id,
            accounts: to_client_metas(&accounts),
            data: family_vault::instruction::AddBeneficiary {
                name: name.to_string(),
                address: to_anchor(*family),
            }
            .data(),
        });
    }

    ixs
}

/// Reads the approvers account (which may not exist yet) and the
/// beneficiary list in one RPC round trip. `get_multiple_accounts`
/// distinguishes "account does not exist" (`None`) from an RPC failure
/// (`Err`), which `get_account_data` cannot.
pub fn fetch_membership_state(
    rpc: &RpcClient,
    approvers: &Pubkey,
    beneficiary_list: &Pubkey,
) -> Result<MembershipState, ChainError> {
    let accounts = rpc
        .get_multiple_accounts(&[*approvers, *beneficiary_list])
        .map_err(map_client_error_pub)?;
    let [approvers_account, list_account] = accounts.as_slice() else {
        return Err(ChainError::ProgramRejected(
            "unexpected get_multiple_accounts response".to_string(),
        ));
    };

    let approvers = match approvers_account {
        Some(account) => Some(
            TrustedApprovers::try_deserialize(&mut &account.data[..])
                .map_err(|e| ChainError::ProgramRejected(e.to_string()))?,
        ),
        None => None,
    };
    let Some(list_account) = list_account else {
        return Err(ChainError::ProgramRejected(
            "beneficiary list account does not exist".to_string(),
        ));
    };
    let beneficiaries = BeneficiaryList::try_deserialize(&mut &list_account.data[..])
        .map_err(|e| ChainError::ProgramRejected(e.to_string()))?;

    Ok(MembershipState {
        approvers,
        beneficiaries,
    })
}

/// Registers `family` (named `name`) as approver and payee of `vault`,
/// submitting only what is still missing. Safe to call repeatedly.
pub fn add_first_family_member(
    rpc: &RpcClient,
    owner: &Keypair,
    program_id: &Pubkey,
    vault: &Pubkey,
    family: &Pubkey,
    name: &str,
) -> Result<(), ChainError> {
    let (approvers, _) = approvers_pda(program_id, vault);
    let (beneficiary_list, _) = beneficiary_list_pda(program_id, vault);

    let state = fetch_membership_state(rpc, &approvers, &beneficiary_list)?;
    let steps = plan_member_steps(state.approvers.as_ref(), &state.beneficiaries, family);
    if steps.is_empty() {
        return Ok(());
    }

    let ctx = MemberContext {
        program_id: *program_id,
        owner: owner.pubkey(),
        vault: *vault,
    };
    let ixs = build_member_instructions(&ctx, &steps, family, name);

    let blockhash = rpc.get_latest_blockhash().map_err(map_client_error_pub)?;
    let tx = solana_transaction::Transaction::new_signed_with_payer(
        &ixs,
        Some(&owner.pubkey()),
        &[owner],
        blockhash,
    );
    rpc.send_and_confirm_transaction(&tx)
        .map_err(map_client_error_pub)?;
    Ok(())
}
```

- [ ] **Step 8: Run to verify everything passes**

Run: `cargo test --package kiosk-gui 2>&1 | grep -E "test result|FAILED|error" ; cargo clippy --workspace --all-targets 2>&1 | tail -10`
Expected: family_member tests `7 passed`; whole `kiosk-gui` unit tests PASS; navigation `10 passed`; clippy clean workspace-wide. If a `TrustedApprovers`/`BeneficiaryList` struct literal in the tests fails to compile because of an extra field, mirror the existing literal in `chain.rs`'s `decodes_a_vault_and_beneficiary_list_into_a_snapshot` test.

- [ ] **Step 9: Commit**

```bash
git add tools/family-vault-client kiosk-gui/src
git commit -m "feat(kiosk-gui): idempotent on-chain registration of the first family member

Reads on-chain membership first and submits only what is missing
(init approvers, add approver, add payee) as one atomic transaction, so
a crash/retry mid-setup cannot double-add. The funding floor now includes
the approvers account's rent. Hoists the duplicated anchor->client
AccountMeta conversion into family-vault-client::to_client_metas.

Claude-Session: https://claude.ai/code/session_01VLZAgeertuGTkdt8LXpiEJ"
```

---

### Task 4: BLE peripheral adapter (`bluer`)

**Files:**
- Create: `kiosk-gui/src/ble.rs`
- Modify: `kiosk-gui/Cargo.toml` (dependencies), `kiosk-gui/src/main.rs` (declare the module)

**Interfaces:**
- Consumes (Tasks 1/2): `family_vault_client::pairing::{PairingRequest, PAIRING_CHARACTERISTIC_UUID_U128, SERVICE_UUID_U128, encode_frame}`, `crate::pairing::{PairingCredentials, PairingSession, WriteOutcome, generate_credentials}`.
- Produces (module `crate::ble`):
  - `struct BleError(pub String)` (`Debug`, `Display`, `Error`)
  - `type CredentialsRotated = std::sync::Arc<dyn Fn(&PairingCredentials) + Send + Sync>`
  - `fn wait_for_pairing(credentials: PairingCredentials, on_rotated: CredentialsRotated, cancel: &std::sync::atomic::AtomicBool) -> Result<Option<PairingRequest>, BleError>` — blocks the calling thread; `Ok(Some(_))` = a request with valid credentials arrived; `Ok(None)` = `cancel` was set; `Err(_)` = Bluetooth unavailable.

- [ ] **Step 1: Dependencies and module declaration**

In `kiosk-gui/Cargo.toml`, directly below the `toml = "1.1.6"` line add exactly:

```toml
bluer = { version = "0.17.4", features = ["bluetoothd"] }
tokio = { version = "1", features = ["rt", "time", "macros", "sync"] }
futures = "0.3"
```

In `kiosk-gui/src/main.rs` add next to the other `mod` lines:

```rust
#[allow(dead_code)] // used starting in Task 6
mod ble;
```

- [ ] **Step 2: Write the failing tests**

Create `kiosk-gui/src/ble.rs` containing ONLY this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use family_vault_client::pairing::{auth_from_pin, encode_frame, AuthField};
    use solana_pubkey::Pubkey;
    use std::sync::atomic::AtomicBool;

    fn creds() -> PairingCredentials {
        PairingCredentials::from_random_bytes([1u8; 20])
    }

    fn frame(auth: AuthField) -> Vec<u8> {
        encode_frame(&PairingRequest {
            auth,
            pubkey: Pubkey::new_from_array([7u8; 32]),
            name: "Внук Саша".to_string(),
        })
        .unwrap()
    }

    fn wrong() -> AuthField {
        [0xEE; 16]
    }

    struct Fixture {
        session: Mutex<PairingSession>,
        tx: mpsc::Sender<PairingRequest>,
        rx: mpsc::Receiver<PairingRequest>,
        callback: CredentialsRotated,
        rotated: Arc<Mutex<Vec<PairingCredentials>>>,
    }

    fn fixture() -> Fixture {
        let rotated = Arc::new(Mutex::new(Vec::new()));
        let sink = rotated.clone();
        let callback: CredentialsRotated =
            Arc::new(move |c: &PairingCredentials| sink.lock().unwrap().push(c.clone()));
        let (tx, rx) = mpsc::channel();
        Fixture {
            session: Mutex::new(PairingSession::new(creds())),
            tx,
            rx,
            callback,
            rotated,
        }
    }

    fn write(f: &Fixture, value: &[u8]) -> Result<(), ReqError> {
        handle_write(&f.session, &f.tx, &f.callback, value)
    }

    #[test]
    fn a_correct_qr_secret_is_acknowledged_and_forwarded() {
        let f = fixture();
        assert!(write(&f, &frame(*creds().secret())).is_ok());
        let request = f.rx.try_recv().expect("accepted request must be forwarded");
        assert_eq!(request.name, "Внук Саша");
    }

    #[test]
    fn a_correct_fallback_code_is_acknowledged_and_forwarded() {
        let f = fixture();
        let field = auth_from_pin(creds().pin()).unwrap();
        assert!(write(&f, &frame(field)).is_ok());
        assert!(f.rx.try_recv().is_ok());
    }

    #[test]
    fn a_partial_write_is_acknowledged_but_not_forwarded() {
        let f = fixture();
        let full = frame(*creds().secret());
        assert!(write(&f, &full[..20]).is_ok());
        assert!(f.rx.try_recv().is_err());
    }

    #[test]
    fn wrong_credentials_are_rejected_with_not_authorized_and_not_forwarded() {
        let f = fixture();
        let res = write(&f, &frame(wrong()));
        assert!(matches!(res, Err(ReqError::NotAuthorized)));
        assert!(f.rx.try_recv().is_err());
    }

    #[test]
    fn a_malformed_frame_is_rejected_with_invalid_value_length() {
        let f = fixture();
        let mut bad = frame(*creds().secret());
        bad.push(0); // trailing byte
        let res = write(&f, &bad);
        assert!(matches!(res, Err(ReqError::InvalidValueLength)));
    }

    #[test]
    fn lockout_rotates_the_credentials_and_tells_the_ui() {
        let f = fixture();
        for _ in 0..5 {
            let res = write(&f, &frame(wrong()));
            assert!(matches!(res, Err(ReqError::NotAuthorized)));
        }
        let new_creds = {
            let seen = f.rotated.lock().unwrap();
            assert_eq!(seen.len(), 1, "UI must be told about exactly one rotation");
            seen[0].clone()
        };
        assert_ne!(new_creds.secret(), creds().secret());

        // The old QR secret is dead; the freshly shown one works.
        assert!(matches!(
            write(&f, &frame(*creds().secret())),
            Err(ReqError::NotAuthorized)
        ));
        assert!(write(&f, &frame(*new_creds.secret())).is_ok());
        assert!(f.rx.try_recv().is_ok());
    }

    // The two tests below register a real advertisement and GATT
    // application with BlueZ, so they need a Bluetooth adapter and a running
    // bluetoothd. Run manually:
    //   cargo test -p kiosk-gui ble::tests -- --ignored --nocapture
    #[test]
    #[ignore = "needs a Bluetooth adapter and a running bluetoothd"]
    fn returns_none_immediately_when_cancel_is_already_set() {
        let cancel = AtomicBool::new(true);
        let out = wait_for_pairing(creds(), Arc::new(|_| {}), &cancel).unwrap();
        assert_eq!(out, None);
    }

    #[test]
    #[ignore = "needs a Bluetooth adapter and a running bluetoothd"]
    fn stops_when_cancel_is_set_while_advertising() {
        let cancel = AtomicBool::new(false);
        let started = std::time::Instant::now();
        std::thread::scope(|s| {
            s.spawn(|| {
                std::thread::sleep(Duration::from_millis(1500));
                cancel.store(true, Ordering::Relaxed);
            });
            let out = wait_for_pairing(creds(), Arc::new(|_| {}), &cancel).unwrap();
            assert_eq!(out, None);
        });
        assert!(started.elapsed() < Duration::from_secs(10));
    }
}
```

- [ ] **Step 3: Run to verify failure**

Run: `cargo test --package kiosk-gui ble::tests 2>&1 | tail -10`
Expected: compile FAIL — `handle_write`, `wait_for_pairing`, `CredentialsRotated`, `ReqError`, `Mutex`, `mpsc`, `Arc`, `Duration`, `Ordering`, `PairingRequest`, `PairingSession`, `PairingCredentials` not found.

- [ ] **Step 4: Write the implementation**

Insert at the TOP of `kiosk-gui/src/ble.rs`:

```rust
//! BLE peripheral side of first-run family-member pairing (Linux/BlueZ only,
//! via `bluer`). While `wait_for_pairing` runs, the kiosk advertises one
//! GATT service with one writable characteristic; a family member's app
//! writes a frame (see `family_vault_client::pairing`) and the kiosk accepts
//! it only if it carries the QR secret or fallback code currently shown on
//! screen. All the decision-making lives in
//! `crate::pairing::PairingSession`; this module only adapts it to BlueZ and
//! maps outcomes to ATT errors.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use bluer::adv::Advertisement;
use bluer::gatt::local::{
    Application, Characteristic, CharacteristicWrite, CharacteristicWriteMethod, ReqError, Service,
};
use bluer::Uuid;
use family_vault_client::pairing::{
    PairingRequest, PAIRING_CHARACTERISTIC_UUID_U128, SERVICE_UUID_U128,
};
use futures::FutureExt;

use crate::pairing::{generate_credentials, PairingCredentials, PairingSession, WriteOutcome};

const SERVICE_UUID: Uuid = Uuid::from_u128(SERVICE_UUID_U128);
const PAIRING_CHARACTERISTIC_UUID: Uuid = Uuid::from_u128(PAIRING_CHARACTERISTIC_UUID_U128);

/// Called (from the BLE runtime thread) with the new credentials whenever
/// the kiosk rotates them after too many wrong attempts, so the UI can
/// redraw the QR and code.
pub type CredentialsRotated = Arc<dyn Fn(&PairingCredentials) + Send + Sync>;

/// Bluetooth is not usable (no adapter, `bluetoothd` not running, ...).
/// The setup flow treats this as "skip the pairing step", never as fatal.
#[derive(Debug)]
pub struct BleError(pub String);

impl fmt::Display for BleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bluetooth unavailable: {}", self.0)
    }
}

impl std::error::Error for BleError {}

fn unavailable(e: bluer::Error) -> BleError {
    BleError(e.to_string())
}

/// Blocks the calling thread while advertising, until either a request
/// carrying valid credentials arrives (`Ok(Some(request))`) or `cancel` is
/// set (`Ok(None)`). The advertisement and GATT application are removed
/// before this returns.
pub fn wait_for_pairing(
    credentials: PairingCredentials,
    on_rotated: CredentialsRotated,
    cancel: &AtomicBool,
) -> Result<Option<PairingRequest>, BleError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| BleError(e.to_string()))?;
    runtime.block_on(run(credentials, on_rotated, cancel))
}

async fn run(
    credentials: PairingCredentials,
    on_rotated: CredentialsRotated,
    cancel: &AtomicBool,
) -> Result<Option<PairingRequest>, BleError> {
    let session = bluer::Session::new().await.map_err(unavailable)?;
    let adapter = session.default_adapter().await.map_err(unavailable)?;
    adapter.set_powered(true).await.map_err(unavailable)?;

    let advertisement = Advertisement {
        service_uuids: vec![SERVICE_UUID].into_iter().collect(),
        discoverable: Some(true),
        local_name: Some("FamilyVault".to_string()),
        ..Default::default()
    };
    let advertisement_handle = adapter.advertise(advertisement).await.map_err(unavailable)?;

    let state = Arc::new(Mutex::new(PairingSession::new(credentials)));
    let (accepted_tx, accepted_rx) = mpsc::channel::<PairingRequest>();

    let write_state = state.clone();
    let write_tx = accepted_tx.clone();
    let write_rotated = on_rotated.clone();
    let application = Application {
        services: vec![Service {
            uuid: SERVICE_UUID,
            primary: true,
            characteristics: vec![Characteristic {
                uuid: PAIRING_CHARACTERISTIC_UUID,
                write: Some(CharacteristicWrite {
                    write: true,
                    write_without_response: true,
                    method: CharacteristicWriteMethod::Fun(Box::new(move |value, _req| {
                        let state = write_state.clone();
                        let tx = write_tx.clone();
                        let rotated = write_rotated.clone();
                        async move { handle_write(&state, &tx, &rotated, &value) }.boxed()
                    })),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        }],
        ..Default::default()
    };
    let application_handle = adapter
        .serve_gatt_application(application)
        .await
        .map_err(unavailable)?;

    let outcome = loop {
        if cancel.load(Ordering::Relaxed) {
            break None;
        }
        match accepted_rx.try_recv() {
            Ok(request) => break Some(request),
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => break None,
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    };

    // Tear down explicitly and give BlueZ a moment to process the removal
    // before the runtime (and with it the D-Bus connection) goes away.
    drop(application_handle);
    drop(advertisement_handle);
    tokio::time::sleep(Duration::from_millis(300)).await;

    Ok(outcome)
}

/// Maps one BLE write to an ATT response. Wrong credentials and malformed
/// frames get a real ATT error so the central can tell the user what went
/// wrong. Never logs the secret or the code.
fn handle_write(
    session: &Mutex<PairingSession>,
    accepted: &mpsc::Sender<PairingRequest>,
    on_rotated: &CredentialsRotated,
    value: &[u8],
) -> Result<(), ReqError> {
    let Ok(mut session) = session.lock() else {
        return Err(ReqError::Failed);
    };
    let outcome = session.on_write(value, Instant::now(), || generate_credentials().ok());
    match outcome {
        WriteOutcome::NeedMore => Ok(()),
        WriteOutcome::Accepted(request) => {
            let _ = accepted.send(request);
            Ok(())
        }
        WriteOutcome::WrongAuth { attempts_left } => {
            eprintln!("[kiosk-gui] pairing: wrong credentials ({attempts_left} attempts left)");
            Err(ReqError::NotAuthorized)
        }
        WriteOutcome::LockedOut { new_credentials } => {
            eprintln!("[kiosk-gui] pairing: too many wrong attempts, rotating credentials");
            if let Some(credentials) = new_credentials {
                on_rotated(&credentials);
            }
            Err(ReqError::NotAuthorized)
        }
        WriteOutcome::Malformed(e) => {
            eprintln!("[kiosk-gui] pairing: malformed frame: {e:?}");
            Err(ReqError::InvalidValueLength)
        }
    }
}
```

- [ ] **Step 5: Run the automated tests**

Run: `cargo test --package kiosk-gui ble::tests 2>&1 | tail -15`
Expected: `6 passed; 0 failed; 2 ignored`.

- [ ] **Step 6: Run the hardware tests on this machine (it has `hci0` and a running `bluetooth` service)**

Run: `cargo test --package kiosk-gui ble::tests -- --ignored --nocapture 2>&1 | tail -15`
Expected: `2 passed`. These prove the advertisement + GATT application really register with BlueZ and are removed on cancel. If they fail with a D-Bus/permission error, report it — do not delete the tests.

- [ ] **Step 7: Lint and commit**

Run: `cargo clippy --workspace --all-targets 2>&1 | tail -10` — expected: clean.

```bash
git add kiosk-gui/Cargo.toml kiosk-gui/src Cargo.lock
git commit -m "feat(kiosk-gui): BLE peripheral adapter for family-member pairing

bluer GATT server with one writable characteristic; advertises only while
wait_for_pairing runs and tears down before returning. Wrong credentials
and malformed frames surface as real ATT errors. Verified against a real
BlueZ (advertisement + GATT application register and are removed).

Claude-Session: https://claude.ai/code/session_01VLZAgeertuGTkdt8LXpiEJ"
```

---

### Task 5: Pairing view on the setup screen

**Files:**
- Modify: `kiosk-gui/ui/theme.slint`, `kiosk-gui/ui/setup.slint`, `kiosk-gui/ui/app.slint`
- Modify: `kiosk-gui/tests/navigation.rs`

**Interfaces:**
- Produces (Slint, exposed on `AppWindow`, so Rust gets these generated members): `set_setup_pairing_active(bool)`, `set_setup_pin_text(SharedString)`, `on_skip_pairing_requested(impl FnMut())`. The existing `set_setup_qr_image` shows the pairing QR while pairing is active (Rust swaps the image), and `set_setup_status_text` doubles as the instruction line.
- The skip button has `accessible-role: button` and `accessible-label: "Пропустить"` and exists in the element tree ONLY while `pairing-active` is true.

- [ ] **Step 1: Write the failing tests**

In `kiosk-gui/tests/navigation.rs`, add these two functions above `fn main()`:

```rust
fn skip_button_exists_only_while_pairing_is_active() {
    slint::spawn_local(async move {
        let app = AppWindow::new().unwrap();
        app.set_current_screen(Screen::Setup);

        assert!(
            ElementHandle::find_by_accessible_label(&app, "Пропустить")
                .next()
                .is_none(),
            "skip button must not exist outside pairing mode"
        );

        app.set_setup_pairing_active(true);
        assert!(
            ElementHandle::find_by_accessible_label(&app, "Пропустить")
                .any(|e| e.accessible_role() == Some(AccessibleRole::Button)),
            "skip button must exist as a button in pairing mode"
        );

        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}

fn tapping_skip_fires_skip_pairing_requested() {
    slint::spawn_local(async move {
        let app = AppWindow::new().unwrap();
        app.set_current_screen(Screen::Setup);
        app.set_setup_pairing_active(true);
        app.set_setup_pin_text("482 913".into());

        let fired = std::rc::Rc::new(std::cell::Cell::new(false));
        let fired_clone = fired.clone();
        app.on_skip_pairing_requested(move || fired_clone.set(true));

        let button = ElementHandle::find_by_accessible_label(&app, "Пропустить")
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .expect("skip button not found");
        button.single_click(PointerEventButton::Left).await;

        assert!(fired.get());

        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}
```

In `fn main()`, add before the final `println!();`:

```rust
    skip_button_exists_only_while_pairing_is_active();
    println!("test skip_button_exists_only_while_pairing_is_active ... ok");

    tapping_skip_fires_skip_pairing_requested();
    println!("test tapping_skip_fires_skip_pairing_requested ... ok");
```

and change the summary line's `10 passed` to `12 passed`.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --package kiosk-gui --test navigation 2>&1 | tail -12`
Expected: compile FAIL — `set_setup_pairing_active` / `set_setup_pin_text` / `on_skip_pairing_requested` not found.

- [ ] **Step 3: Implement the UI**

In `kiosk-gui/ui/theme.slint`, add below `font-size-body`:

```slint

    // The fallback pairing code must be read and typed correctly, so it
    // gets the largest type on its screen.
    out property <length> font-size-code: 64px;
```

Replace the entire contents of `kiosk-gui/ui/setup.slint` with:

```slint
import { Theme } from "theme.slint";

export component SetupScreen inherits Rectangle {
    // In funding mode this is the funding QR; in pairing mode Rust swaps in
    // the pairing QR (the same property, so the Image element is shared).
    in property <image> qr-image;
    in property <string> status-text: "";

    // Pairing mode: QR on the left, and on the right the instruction, the
    // fallback code for people who cannot scan, and a way to skip. The
    // kiosk has no input device for this step, so skipping is the only
    // control. See the family-app design spec, "Identity & Bootstrap".
    in property <bool> pairing-active: false;
    in property <string> pin-text: "";

    callback skip-pairing-requested();

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

        if !pairing-active : VerticalLayout {
            spacing: Theme.spacing;

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

        if pairing-active : HorizontalLayout {
            spacing: Theme.spacing;

            Image {
                source: qr-image;
                width: 280px;
                height: 280px;
            }

            VerticalLayout {
                spacing: Theme.spacing;
                alignment: center;

                Text {
                    text: status-text;
                    color: Theme.fg;
                    font-size: Theme.font-size-body;
                    wrap: word-wrap;
                }

                Text {
                    text: pin-text;
                    color: Theme.fg;
                    font-size: Theme.font-size-code;
                    font-weight: 700;
                }

                Rectangle {
                    height: Theme.target-size;
                    background: Theme.secondary-button-bg;
                    border-color: Theme.secondary-button-border;
                    border-width: 2px;
                    border-radius: 8px;
                    accessible-role: button;
                    accessible-label: "Пропустить";

                    TouchArea {
                        clicked => { skip-pairing-requested(); }
                    }

                    Text {
                        text: "Пропустить";
                        color: Theme.secondary-button-fg;
                        font-size: Theme.font-size-body;
                        horizontal-alignment: center;
                        vertical-alignment: center;
                    }
                }
            }
        }
    }
}
```

In `kiosk-gui/ui/app.slint`, add next to the existing `setup-*` properties:

```slint
    in property <bool> setup-pairing-active <=> setup.pairing-active;
    in property <string> setup-pin-text <=> setup.pin-text;
```

and next to the other `callback ... <=> ...;` lines:

```slint
    callback skip-pairing-requested <=> setup.skip-pairing-requested;
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --package kiosk-gui --test navigation 2>&1 | tail -18`
Expected: `12 passed; 0 failed`. If `if cond : Element` inside the layout is rejected by the Slint compiler, restructure using a `visible:`-free alternative of your choice that keeps the same guarantee (the skip button must NOT be in the element tree unless `pairing-active`); keep the accessible label/role unchanged.

- [ ] **Step 5: Check the screen fits 800x480 and commit**

If a display is available, look at both modes; otherwise reason from the numbers. Pairing mode: heading 40 + 16 gap + max(QR 280, right column ≈ instruction 2-3 lines of 28px ≈ 100 + code 64 + button 60 + 2 gaps 32 ≈ 256) + padding 32 ≈ 370 px < 480 px; width: 280 + 16 + right column ≈ 470 px, the code «482 913» at 64 px ≈ 300 px. Funding mode is unchanged (heading + 280 QR + status).

Run: `cargo clippy --workspace --all-targets 2>&1 | tail -6` — expected: clean.

```bash
git add kiosk-gui/ui kiosk-gui/tests/navigation.rs
git commit -m "feat(kiosk-gui): pairing view on the setup screen

Pairing mode shows the pairing QR on the left and, on the right, the
instruction, the large fallback code and a single Пропустить button. The
skip button only exists in the element tree in pairing mode.

Claude-Session: https://claude.ai/code/session_01VLZAgeertuGTkdt8LXpiEJ"
```

---

### Task 6: Wire pairing into the first-run setup flow

**Files:**
- Modify: `kiosk-gui/src/main.rs`

**Interfaces:**
- Consumes: everything above. `setup::ProvisioningConfig { rpc_url, program_id: AnchorPubkey, mint }`, `setup::render_qr(&str) -> slint::Image`, the existing `run_setup_flow`, `set_setup_status`.
- Produces: `fn show_pairing_view`, `fn hide_pairing_view`, `fn run_family_pairing_step(...)`; `run_setup_flow` gains a `skip_pairing: Arc<AtomicBool>` parameter.

- [ ] **Step 1: Remove the three temporary `allow(dead_code)` markers**

Delete these attribute lines (and their `// used starting in Task 6` comments) above `mod pairing;`, `mod family_member;`, `mod ble;` in `kiosk-gui/src/main.rs`, leaving plain `mod ble; mod family_member; mod pairing;`.

Run: `cargo check --package kiosk-gui 2>&1 | tail -20`
Expected: `dead_code` errors for every item not yet used by `main.rs` (this is the red step: it lists exactly what must be wired). `generate_credentials`, `display_pin`, `PairingCredentials::{qr_payload, pin}`, `ble::wait_for_pairing`, `ble::CredentialsRotated`, `family_member::{fetch_membership_state, needs_pairing, add_first_family_member}` must all end up used; the rest is used transitively. Anything still reported after Step 3 that is genuinely unused (for example `PIN_LEN`-style constants or `FrameAssembler` re-exports) must be deleted or made non-`pub`, not re-allowed.

- [ ] **Step 2: Add the pairing step functions**

In `kiosk-gui/src/main.rs`, add directly below `fn set_setup_status(...)`:

```rust
/// Switches the setup screen into pairing mode: the pairing QR, the
/// instruction and the fallback code. Safe to call from any thread — the
/// `slint::Image` is built inside the UI-thread closure because it is not
/// `Send`. Called again when the kiosk rotates its credentials.
fn show_pairing_view(
    app_weak: &slint::Weak<AppWindow>,
    credentials: &pairing::PairingCredentials,
) {
    let app_weak = app_weak.clone();
    let qr_text = credentials.qr_payload();
    let code = pairing::display_pin(credentials.pin());
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(app) = app_weak.upgrade() {
            app.set_setup_qr_image(setup::render_qr(&qr_text));
            app.set_setup_status_text(
                "Отсканируйте этот код в приложении семьи или введите цифры:".into(),
            );
            app.set_setup_pin_text(code.into());
            app.set_setup_pairing_active(true);
        }
    });
}

/// Leaves pairing mode and puts the funding QR back (which is what the
/// screen showed for the rest of setup before pairing existed).
fn hide_pairing_view(app_weak: &slint::Weak<AppWindow>, funding_qr_text: &str) {
    let app_weak = app_weak.clone();
    let funding_qr_text = funding_qr_text.to_string();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(app) = app_weak.upgrade() {
            app.set_setup_pairing_active(false);
            app.set_setup_pin_text("".into());
            app.set_setup_qr_image(setup::render_qr(&funding_qr_text));
        }
    });
}

/// First-run step between "personal vault exists" and "write kiosk.toml":
/// let one family member hand the kiosk their key over Bluetooth and
/// register them on-chain. Never fatal — no Bluetooth, a skip, or a chain
/// failure all just continue setup (the first family member can then be
/// added with `admin-cli`). Runs BEFORE `kiosk.toml` is written and is
/// idempotent, so a power cut anywhere in here resumes cleanly.
fn run_family_pairing_step(
    app_weak: &slint::Weak<AppWindow>,
    rpc: &solana_client::rpc_client::RpcClient,
    owner: &solana_keypair::Keypair,
    provisioning: &setup::ProvisioningConfig,
    vault: &solana_pubkey::Pubkey,
    skip_pairing: &Arc<std::sync::atomic::AtomicBool>,
) {
    let program_id = family_vault_client::to_client(provisioning.program_id);
    let (approvers_pk, _) = family_vault_client::approvers_pda(&program_id, vault);
    let (list_pk, _) = family_vault_client::beneficiary_list_pda(&program_id, vault);

    match family_member::fetch_membership_state(rpc, &approvers_pk, &list_pk) {
        Ok(state) if !family_member::needs_pairing(&state) => {
            eprintln!("[kiosk-gui] setup: a family member is already registered, skipping pairing");
            return;
        }
        Ok(_) => {}
        Err(e) => eprintln!(
            "[kiosk-gui] setup: could not read membership state ({e:?}); offering pairing anyway"
        ),
    }

    let credentials = match pairing::generate_credentials() {
        Ok(credentials) => credentials,
        Err(e) => {
            eprintln!(
                "[kiosk-gui] setup: could not generate pairing credentials ({e}); skipping pairing"
            );
            return;
        }
    };

    show_pairing_view(app_weak, &credentials);
    let on_rotated: ble::CredentialsRotated = {
        let app_weak = app_weak.clone();
        Arc::new(move |new_credentials: &pairing::PairingCredentials| {
            show_pairing_view(&app_weak, new_credentials)
        })
    };
    let outcome = ble::wait_for_pairing(credentials, on_rotated, skip_pairing.as_ref());
    hide_pairing_view(app_weak, &solana_signer::Signer::pubkey(owner).to_string());

    let request = match outcome {
        Ok(Some(request)) => request,
        Ok(None) => {
            eprintln!("[kiosk-gui] setup: family pairing skipped");
            return;
        }
        Err(e) => {
            eprintln!("[kiosk-gui] setup: {e}; skipping family pairing");
            return;
        }
    };

    set_setup_status(app_weak, "Добавляем участника семьи...".to_string());
    let mut added = false;
    for attempt in 0..5u32 {
        match family_member::add_first_family_member(
            rpc,
            owner,
            &program_id,
            vault,
            &request.pubkey,
            &request.name,
        ) {
            Ok(()) => {
                eprintln!(
                    "[kiosk-gui] setup: registered family member {} (\"{}\")",
                    request.pubkey, request.name
                );
                added = true;
                break;
            }
            Err(e) => {
                eprintln!("[kiosk-gui] setup: add family member attempt {attempt} failed: {e:?}");
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        }
    }
    if !added {
        eprintln!(
            "[kiosk-gui] setup: could not register the family member; add them later with admin-cli"
        );
    }
    set_setup_status(app_weak, "Почти готово...".to_string());
}
```

- [ ] **Step 3: Thread the skip flag and call the step**

In `main()`, immediately before the `if setup::needs_setup(&config_path) {` block, add:

```rust
    // Set by the setup screen's "Пропустить" button; read by the BLE
    // pairing step running on the setup thread.
    let skip_pairing = Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let skip_pairing = skip_pairing.clone();
        app.on_skip_pairing_requested(move || {
            skip_pairing.store(true, std::sync::atomic::Ordering::Relaxed);
        });
    }
```

Change the call inside that block to `run_setup_flow(app.as_weak(), config_path.clone(), cfg_slot, skip_pairing.clone());`, and change `run_setup_flow`'s signature to:

```rust
fn run_setup_flow(
    app_weak: slint::Weak<AppWindow>,
    config_path: std::path::PathBuf,
    cfg_slot: Arc<Mutex<Option<Arc<ChainConfig>>>>,
    skip_pairing: Arc<std::sync::atomic::AtomicBool>,
) {
```

Inside the setup thread, immediately after the `let Some((vault, beneficiary_list)) = vault_result else { ... };` block and BEFORE `let values = setup::KioskConfigValues {`, add:

```rust
        run_family_pairing_step(&app_weak, &rpc, &owner, &provisioning, &vault, &skip_pairing);
```

- [ ] **Step 4: Verify**

Run:
```bash
cargo check --package kiosk-gui 2>&1 | tail -10
cargo test --package kiosk-gui 2>&1 | grep -E "test result|FAILED|error"
cargo clippy --workspace --all-targets 2>&1 | tail -10
cargo test --workspace 2>&1 | grep -E "test result|FAILED"
```
Expected: no `dead_code` errors (nothing left un-consumed), all tests pass (family-vault program tests unchanged and green, navigation `12 passed`), clippy clean workspace-wide.

- [ ] **Step 5: Commit**

```bash
git add kiosk-gui/src/main.rs
git commit -m "feat(kiosk-gui): pair the first family member during first-run setup

After the personal vault exists and before kiosk.toml is written, show a
pairing QR (plus a typed fallback code), accept one family member over
Bluetooth, and register them as approver and payee in one atomic
transaction. Never fatal: no Bluetooth, a skip, or a chain failure just
continues setup. Idempotent, so a power cut anywhere in the step resumes
cleanly.

Claude-Session: https://claude.ai/code/session_01VLZAgeertuGTkdt8LXpiEJ"
```

---

## Manual verification (human, after Task 6 — not automatable here)

Needs a funded devnet setup, a phone with a BLE tool (e.g. nRF Connect) and a QR scanner app, and a fresh config dir with a valid `provisioning.toml`.

1. Start the kiosk on a fresh config dir; fund the QR address as in the first-run-setup flow. Expected: after the vault initialises, the screen shows a **new QR** on the left and on the right «Отсканируйте этот код в приложении семьи или введите цифры:», a large `NNN NNN` code, and a «Пропустить» button.
2. Scan the QR with any phone scanner; it should read `fvpair1:` followed by 32 hex characters.
3. Generate a frame on the dev machine, QR path: `PAIR_QR='<scanned text>' PAIR_PUBKEY=<any base58 pubkey> PAIR_NAME="Внук Саша" cargo test -p family-vault-client print_frame_hex -- --ignored --nocapture`. (Code path: use `PAIR_PIN=<the digits, no space>` instead of `PAIR_QR`.)
4. In nRF Connect: scan, connect to **FamilyVault**, open the service `8f1a0000-5b1e-4c6a-9a7e-0d2f3a1b4c5d`, write the printed hex (as bytes) to characteristic `…0001…`. Expected: write succeeds; kiosk shows «Добавляем участника семьи...» then continues to the wallet screen.
5. Wrong-credentials check: write a frame with a wrong `PAIR_PIN` — expected ATT error "insufficient authorization"; after 5 wrong writes the on-screen QR and code change.
6. Inspect the log for `registered family member <pubkey> ("Внук Саша")`, then confirm on-chain (`solana account <approvers PDA> --url devnet`) that the pubkey is listed, and that the kiosk's payee list now shows «Внук Саша» (active after the 48 h cooldown).
7. Resume check: kill the kiosk right after step 4, restart it. Expected: no second pairing prompt (an approver already exists) and setup completes.
8. Skip check: on a fresh config dir tap «Пропустить». Expected: setup completes normally with no family member.
