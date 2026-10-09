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
use crate::state::{AppState, BusyGuard};
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

/// Reads the pending record. A corrupt one is moved aside (never deleted)
/// and reported once, so it cannot block every money command forever.
pub fn load_pending(state: &AppState) -> Result<Option<PendingTx>, ErrorDto> {
    let path = state.pending_path();
    match pending::load(&path) {
        Ok(record) => Ok(record),
        Err(pending::LoadError::Io(e)) => Err(ErrorDto::new(
            "internal",
            "Не удалось прочитать записи приложения.",
        )
        .with_details(e)),
        Err(pending::LoadError::Corrupt(why)) => match pending::quarantine(&path) {
            Ok(moved) => Err(ErrorDto::new(
                "blocked",
                "Запись о прошлой операции оказалась повреждена, и мы не можем узнать, чем она кончилась. Проверьте историю счёта и свой кошелёк, прежде чем повторять перевод. Запись сохранена отдельно, приложение снова работает.",
            )
            .with_details(moved.display().to_string())),
            Err(e) => Err(ErrorDto::new(
                "internal",
                "Не удалось прочитать записи приложения.",
            )
            .with_details(format!("{why}; {e}"))),
        },
    }
}

/// Only one money command may run at a time: the pending record is written
/// only after a send returns, so without this a double tap could pay twice.
pub fn acquire_busy(state: &AppState) -> Result<BusyGuard, ErrorDto> {
    BusyGuard::acquire(&state.busy)
        .ok_or_else(|| ErrorDto::new("blocked", "Подождите: предыдущая операция ещё выполняется."))
}

/// While an operation's outcome is unknown nothing else may move money.
pub fn ensure_no_pending(state: &AppState) -> Result<(), ErrorDto> {
    match load_pending(state)? {
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
/// Returns why it could not be saved, if so.
pub fn record_unconfirmed(state: &AppState, what: &str, error: &ActionError) -> Option<String> {
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
        return pending::save(&state.pending_path(), &record).err();
    }
    None
}

/// Pure part: if the record could not be saved, tell the person not to
/// repeat the operation and keep the signature and the reason in `details`.
pub fn apply_save_failure(dto: ErrorDto, signature: &str, failure: Option<String>) -> ErrorDto {
    match failure {
        None => dto,
        Some(reason) => ErrorDto {
            kind: dto.kind,
            message: format!(
                "{} Приложение не смогло запомнить эту операцию. НЕ повторяйте её, пока не проверите историю счёта.",
                dto.message
            ),
            details: Some(format!(
                "signature: {signature}; запись не сохранена: {reason}"
            )),
        },
    }
}

/// Converts a failed core call into the command error, recording `Unconfirmed`.
pub fn failure(state: &AppState, what: &str, error: ActionError) -> ErrorDto {
    let failure = record_unconfirmed(state, what, &error);
    let signature = match &error {
        ActionError::Unconfirmed { signature, .. } => Some(signature.to_string()),
        _ => None,
    };
    let dto = ErrorDto::from(error);
    match signature {
        Some(sig) => apply_save_failure(dto, &sig, failure),
        None => dto,
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
        Err(error) => Err(failure(state, what, error)),
    }
}

pub fn not_allowed(text: &str) -> ErrorDto {
    ErrorDto::new("blocked", text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_record_leaves_the_error_unchanged() {
        let dto = ErrorDto::new("unconfirmed", "Не знаем.").with_details("d");
        assert_eq!(apply_save_failure(dto.clone(), "SIG", None), dto);
    }

    #[test]
    fn a_failed_save_warns_and_keeps_signature_and_reason() {
        let dto = apply_save_failure(
            ErrorDto::new("unconfirmed", "Не знаем."),
            "SIG123",
            Some("disk full".to_string()),
        );
        assert!(dto.message.starts_with("Не знаем."));
        assert!(dto.message.contains("НЕ повторяйте"));
        let details = dto.details.unwrap();
        assert!(details.contains("SIG123"));
        assert!(details.contains("disk full"));
    }
}
