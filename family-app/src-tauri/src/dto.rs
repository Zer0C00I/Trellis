//! What crosses the IPC boundary. Plain, serializable. Sentences travel as
//! catalog keys with parameters (`Msg`), worded by the JavaScript side in the
//! person's language; that side never does money math (u64 amounts do not
//! fit a JS number, so they travel as strings that Rust formatted).

use serde::Serialize;

use crate::msg::Msg;

/// A failed command. `kind` is one of `no_connection`, `rejected`,
/// `unconfirmed`, `blocked`, `invalid`, `internal`.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct ErrorDto {
    pub kind: &'static str,
    /// The plain-language sentence for the screen (`key`, `params`).
    #[serde(flatten)]
    pub message: Msg,
    /// Technical text for a "details" fold-out; never the only explanation.
    pub details: Option<String>,
}

impl ErrorDto {
    pub fn new(kind: &'static str, key: &str) -> Self {
        Self::from_msg(kind, Msg::new(key))
    }

    pub fn from_msg(kind: &'static str, message: Msg) -> Self {
        Self {
            kind,
            message,
            details: None,
        }
    }

    pub fn with_msg_param(mut self, name: &str, message: &Msg) -> Self {
        self.message = self.message.with_msg(name, message);
        self
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct AppInfoDto {
    pub address: String,
    pub address_short: String,
    pub network: &'static str,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct WalletDto {
    /// SOL for fees, already formatted ("0.1").
    pub sol: String,
    pub low_sol: bool,
}

/// An entry for the vault picker; cheap to build (no balance lookups).
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct VaultRefDto {
    pub address: String,
    pub address_short: String,
    /// `true` when this key owns the vault (created it).
    pub mine: bool,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct PayeeDto {
    pub name: String,
    pub address: String,
    pub address_short: String,
    pub active: bool,
    /// `payee.ready`, `payee.wait_under_hour` or `payee.wait_hours` (`n`).
    pub status: Msg,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct GuardianDto {
    pub address: String,
    pub address_short: String,
    pub is_me: bool,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct PendingRequestDto {
    pub nonce: u64,
    pub amount: String,
    pub to: String,
    pub to_short: String,
    pub requested_at: i64,
}

/// What this key may do on this vault, decided once in Rust. The screens
/// only show the buttons that are `true`; the commands check again.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionsDto {
    pub deposit: bool,
    pub send: bool,
    pub add_payee: bool,
    pub add_guardian: bool,
    pub approve: bool,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct VaultDto {
    pub address: String,
    pub address_short: String,
    pub is_pool: bool,
    pub is_owner: bool,
    pub is_guardian: bool,
    pub balance: String,
    pub decimals: u32,
    /// `None` when the vault has no daily limit.
    pub daily_limit: Option<String>,
    pub remaining_today: Option<String>,
    pub payees: Vec<PayeeDto>,
    pub guardians: Vec<GuardianDto>,
    pub pending: Vec<PendingRequestDto>,
    pub permissions: PermissionsDto,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct HistoryLineDto {
    pub signature: String,
    /// What happened (`history.*`).
    #[serde(flatten)]
    pub text: Msg,
    /// Unix seconds, `None` if the node does not know the block time.
    pub time: Option<i64>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct ActionOkDto {
    pub signature: String,
    /// What happened (`notice.*`).
    #[serde(flatten)]
    pub message: Msg,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct PendingTxDto {
    /// What the person was doing (`op.*`).
    pub what: Msg,
    pub signature: String,
}

/// `status`: `none`, `waiting`, `confirmed`, `failed`, `expired`.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct ResolutionDto {
    pub status: &'static str,
    /// `resolution.*`
    #[serde(flatten)]
    pub message: Msg,
}
