//! Headless core for the family-facing companion app: local identity,
//! on-chain vault discovery, every everyday action (open an account,
//! deposit, send, add a payee, add a trusted person, approve an
//! over-limit withdrawal), a read-only view of one vault (`vault_view`:
//! balance, limits, payees, guardians, pending requests), read-only
//! transaction history (`history`), and BLE pairing with a kiosk during
//! its first-run setup. No UI, no process arguments, no
//! `std::process::exit` anywhere — every public function returns a
//! `Result` so a future Tauri (or test) caller decides what to do with a
//! failure.
//!
//! Money-moving actions report an unknown outcome as
//! `ActionError::Unconfirmed { signature, last_valid_block_height }`: store
//! both, resolve with `actions::confirm_signature`, and only retry after
//! `Failed` or `Expired`. `actions::request_withdrawal` takes an explicit
//! nonce (from `VaultView::next_free_nonce`) that must be stored and reused
//! on retry.
//!
//! See `docs/superpowers/specs/2026-09-23-family-app-design.md`.

pub mod actions;
pub mod discovery;
pub mod error;
pub mod history;
pub mod identity;
pub mod pairing;
pub mod vault_view;
pub mod wallet;

pub use error::ActionError;
