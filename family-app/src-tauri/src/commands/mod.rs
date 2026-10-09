//! The IPC surface. Each command is thin: check the rules (permissions,
//! pending operation), call one `family-app-core` function on a blocking
//! thread, and phrase the outcome. No command does money math in
//! JavaScript terms: amounts arrive as text and are parsed here, exactly.

pub mod actions;
pub mod pairing;
pub mod read;

use family_app_core::vault_view::{fetch_vault_view, VaultView};
use family_app_core::ActionError;
use solana_pubkey::Pubkey;
use solana_signature::Signature;

use crate::dto::{ActionOkDto, ErrorDto};
use crate::pending::{self, PendingTx};
use crate::state::AppState;
use crate::text::error_dto;

impl From<ActionError> for ErrorDto {
    fn from(error: ActionError) -> Self {
        error_dto(&error)
    }
}

/// Runs blocking RPC/Bluetooth work off the async runtime.
pub async fn blocking<T, F>(work: F) -> Result<T, ErrorDto>
where
    F: FnOnce() -> Result<T, ErrorDto> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| {
            ErrorDto::new("internal", "Что-то пошло не так внутри приложения.")
                .with_details(e.to_string())
        })?
}

pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn parse_pubkey(text: &str, what: &str) -> Result<Pubkey, ErrorDto> {
    text.trim().parse().map_err(|_| {
        ErrorDto::new(
            "invalid",
            format!("{what}: это не похоже на адрес. Проверьте, что он скопирован целиком."),
        )
    })
}

/// Exact decimal parse; `text` is what the person typed ("25", "2.50").
pub fn parse_amount(text: &str, decimals: u32) -> Result<u64, ErrorDto> {
    family_vault_client::parse_decimal_amount(text, decimals).map_err(|e| {
        ErrorDto::new(
            "invalid",
            "Введите сумму числом, например 25 или 2.50 (не больше знаков после точки, чем у денег на счёте).",
        )
        .with_details(e)
    })
}

pub fn load_view(state: &AppState, vault: &str) -> Result<VaultView, ErrorDto> {
    let vault = parse_pubkey(vault, "Счёт")?;
    Ok(fetch_vault_view(&state.rpc, &state.program_id, &vault)?)
}

/// While an operation's outcome is unknown nothing else may move money.
pub fn ensure_no_pending(state: &AppState) -> Result<(), ErrorDto> {
    let waiting = pending::load(&state.pending_path()).map_err(|e| {
        ErrorDto::new("internal", "Не удалось прочитать записи приложения.").with_details(e)
    })?;
    match waiting {
        None => Ok(()),
        Some(p) => Err(ErrorDto::new(
            "blocked",
            format!(
                "Сначала нужно узнать, чем закончилась прошлая операция («{}»). Нажмите «Проверить».",
                p.what
            ),
        )),
    }
}

/// Remembers an unconfirmed operation so the rule above survives a restart.
pub fn record_unconfirmed(state: &AppState, what: &str, error: &ActionError) {
    if let ActionError::Unconfirmed {
        signature,
        last_valid_block_height,
    } = error
    {
        let record = PendingTx {
            signature: signature.to_string(),
            last_valid_block_height: *last_valid_block_height,
            what: what.to_string(),
        };
        // If this fails the user still sees the "unconfirmed" screen; the
        // worst case is that a restart forgets it, never a silent success.
        let _ = pending::save(&state.pending_path(), &record);
    }
}

/// Turns a core result into the command result, recording `Unconfirmed`.
pub fn finish(
    state: &AppState,
    what: &str,
    result: Result<Signature, ActionError>,
    ok_message: &str,
) -> Result<ActionOkDto, ErrorDto> {
    match result {
        Ok(signature) => Ok(ActionOkDto {
            signature: signature.to_string(),
            message: ok_message.to_string(),
        }),
        Err(error) => {
            record_unconfirmed(state, what, &error);
            Err(error.into())
        }
    }
}

pub fn not_allowed(text: &str) -> ErrorDto {
    ErrorDto::new("blocked", text)
}
