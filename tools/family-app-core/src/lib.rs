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
//! See `docs/superpowers/specs/2026-09-23-family-app-design.md`.

pub mod actions;
pub mod discovery;
pub mod error;
pub mod history;
pub mod identity;
pub mod pairing;
pub mod vault_view;

pub use error::ActionError;
