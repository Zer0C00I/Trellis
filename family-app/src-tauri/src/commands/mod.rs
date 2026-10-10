//! The IPC surface. Each command is thin: check the rules (permissions,
//! pending operation), call one `family-app-core` function on a blocking
//! thread, and phrase the outcome. No command does money math in
//! JavaScript terms: amounts arrive as text and are parsed here, exactly.

pub mod actions;
pub mod pairing;
pub mod read;

use family_app_core::actions::BeforeSend;
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
        Err(pending::LoadError::Corrupt(why)) => Err(quarantine_error(state, &why)),
    }
}

/// Moves an unusable pending record aside (never deletes it) and builds the
/// one error the person sees about it.
pub fn quarantine_error(state: &AppState, why: &str) -> ErrorDto {
    match pending::quarantine(&state.pending_path()) {
        Ok(moved) => ErrorDto::new(
            "blocked",
            "Запись о прошлой операции оказалась повреждена, и мы не можем узнать, чем она кончилась. Проверьте историю счёта и свой кошелёк, прежде чем повторять перевод. Запись сохранена отдельно, приложение снова работает.",
        )
        .with_details(moved.display().to_string()),
        Err(e) => ErrorDto::new("internal", "Не удалось прочитать записи приложения.")
            .with_details(format!("{why}; {e}")),
    }
}

/// A record whose signature does not parse is as good as corrupt.
pub fn parse_record_signature(record: &PendingTx) -> Option<Signature> {
    record.signature.parse().ok()
}

/// Whether this key's pool vault is already among its owned vaults.
pub fn pool_already_open(pool: &Pubkey, owned: &[Pubkey]) -> bool {
    owned.contains(pool)
}

/// Only one state-changing command (money, pairing, checking the pending
/// record) may run at a time: a second one starting in the same instant
/// would race with the first one's reads, and a double tap could pay twice.
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

/// Whether the pending record must be forgotten after a core call returned.
/// Only `Unconfirmed` keeps it: every other outcome is settled (it landed,
/// the node refused it, it failed on chain, or nothing was sent at all).
pub fn should_clear<T>(result: &Result<T, ActionError>) -> bool {
    !matches!(result, Err(ActionError::Unconfirmed { .. }))
}

/// Builds the record the hook writes before anything is sent.
pub fn pending_record(
    what: &str,
    signature: &Signature,
    last_valid_block_height: u64,
) -> PendingTx {
    PendingTx {
        signature: signature.to_string(),
        last_valid_block_height,
        what: what.to_string(),
    }
}

/// What the person sees when the record could not be written (so nothing
/// was sent).
pub fn record_failed_dto(reason: String) -> ErrorDto {
    ErrorDto::new(
        "internal",
        "Не удалось записать операцию на этом устройстве, поэтому ничего не было отправлено. Попробуйте ещё раз.",
    )
    .with_details(reason)
}

/// Runs one money operation with the "record before sending" rule: core's
/// before-send hook writes the pending record after signing and before the
/// network is touched, so a crash during the confirm wait cannot lose it.
/// Afterwards the record is cleared unless the outcome is `Unconfirmed`.
pub fn run_money<T>(
    state: &AppState,
    what: &str,
    op: impl FnOnce(BeforeSend) -> Result<T, ActionError>,
) -> Result<T, ErrorDto> {
    let path = state.pending_path();
    let save_error = std::cell::RefCell::new(None::<String>);
    let hook = |signature: &Signature, last_valid_block_height: u64| {
        let record = pending_record(what, signature, last_valid_block_height);
        pending::save(&path, &record).inspect_err(|e| {
            *save_error.borrow_mut() = Some(e.clone());
        })
    };
    let result = op(&hook);
    if let Some(reason) = save_error.into_inner() {
        // Nothing was sent. Do not clear: the file may belong to another
        // operation that the failed save refused to overwrite.
        return Err(record_failed_dto(reason));
    }
    if should_clear(&result) {
        // If this fails the record stays and blocks money commands until
        // "Проверить" settles it: the safe side.
        let _ = pending::clear(&path);
    }
    result.map_err(ErrorDto::from)
}

/// Turns a settled core result into the command result.
pub fn finish(
    result: Result<Signature, ErrorDto>,
    ok_message: &str,
) -> Result<ActionOkDto, ErrorDto> {
    result.map(|signature| ActionOkDto {
        signature: signature.to_string(),
        message: ok_message.to_string(),
    })
}

pub fn not_allowed(text: &str) -> ErrorDto {
    ErrorDto::new("blocked", text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig() -> Signature {
        Signature::from([3u8; 64])
    }

    #[test]
    fn a_record_with_a_bad_signature_is_recognised() {
        let mut r = pending_record("x", &sig(), 1);
        assert_eq!(parse_record_signature(&r), Some(sig()));
        r.signature = "not a signature".to_string();
        assert_eq!(parse_record_signature(&r), None);
    }

    #[test]
    fn an_existing_pool_is_detected() {
        let a = Pubkey::new_from_array([1; 32]);
        let b = Pubkey::new_from_array([2; 32]);
        assert!(pool_already_open(&a, &[a, b]));
        assert!(!pool_already_open(&a, &[b]));
        assert!(!pool_already_open(&a, &[]));
    }

    #[test]
    fn the_record_carries_what_signature_and_height() {
        let r = pending_record("Отправка 5 USDC", &sig(), 77);
        assert_eq!(r.what, "Отправка 5 USDC");
        assert_eq!(r.signature, sig().to_string());
        assert_eq!(r.last_valid_block_height, 77);
    }

    #[test]
    fn the_hook_record_is_written_without_overwriting() {
        let path = std::env::temp_dir().join(format!(
            "family-app-hook-{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        let record = pending_record("x", &sig(), 5);
        pending::save(&path, &record).unwrap();
        assert_eq!(pending::load(&path).unwrap(), Some(record.clone()));
        assert!(pending::save(&path, &pending_record("y", &sig(), 6)).is_err());
        pending::clear(&path).unwrap();
    }

    #[test]
    fn only_unconfirmed_keeps_the_record() {
        assert!(should_clear::<()>(&Ok(())));
        assert!(should_clear::<()>(&Err(ActionError::NoConnection)));
        assert!(should_clear::<()>(&Err(ActionError::Rejected("x".into()))));
        assert!(!should_clear::<()>(&Err(ActionError::Unconfirmed {
            signature: sig(),
            last_valid_block_height: 1,
        })));
    }

    #[test]
    fn a_failed_record_save_says_nothing_was_sent_and_to_retry() {
        let dto = record_failed_dto("disk full".to_string());
        assert_eq!(dto.kind, "internal");
        assert!(dto.message.contains("ничего не было отправлено"));
        assert!(dto.message.contains("Попробуйте ещё раз"));
        assert_eq!(dto.details.as_deref(), Some("disk full"));
    }
}
