//! Every sentence the app shows, and the pure functions that turn core
//! values into them. Plain bank language: no "vault", "PDA", "mint" or
//! "approver" anywhere a person reads it.

use family_app_core::history::HistoryEntryKind;
use family_app_core::pairing::PairingError;
use family_app_core::vault_view::sanitize_text;
use family_app_core::ActionError;
use family_vault::errors::VaultError;
use family_vault_client::format_base_units;
use solana_pubkey::Pubkey;

use family_app_core::actions::SignatureOutcome;

use crate::dto::{ErrorDto, ResolutionDto};

pub fn short_address(address: &Pubkey) -> String {
    let full = address.to_string();
    format!("{}…{}", &full[..4], &full[full.len() - 4..])
}

/// "1 час", "2 часа", "5 часов", "11 часов", "21 час".
pub fn plural_ru(n: u64, one: &str, few: &str, many: &str) -> String {
    let word = match (n % 100, n % 10) {
        (11..=14, _) => many,
        (_, 1) => one,
        (_, 2..=4) => few,
        _ => many,
    };
    format!("{n} {word}")
}

pub fn money(amount: u64, decimals: u32) -> String {
    format!("{} USDC", format_base_units(amount, decimals))
}

pub fn payee_status(active_after: i64, now: i64) -> String {
    if now >= active_after {
        return "Можно отправлять".to_string();
    }
    let secs = active_after.saturating_sub(now) as u64;
    if secs < 3600 {
        return "Можно будет меньше чем через час".to_string();
    }
    let hours = secs.div_ceil(3600);
    format!(
        "Можно будет через {}",
        plural_ru(hours, "час", "часа", "часов")
    )
}

pub fn history_line(kind: &HistoryEntryKind, decimals: u32) -> String {
    // Names in history come straight from the chain: clean them first.
    let quote = |name: &str| format!("«{}»", sanitize_text(name));
    match kind {
        HistoryEntryKind::Deposit { amount } => {
            format!("Пополнение на {}", money(*amount, decimals))
        }
        HistoryEntryKind::Withdraw { amount } => format!("Отправлено {}", money(*amount, decimals)),
        HistoryEntryKind::AddBeneficiary { name } => format!("Добавлен получатель {}", quote(name)),
        HistoryEntryKind::AddBeneficiaryUrgent { name } => {
            format!("Срочно добавлен получатель {}", quote(name))
        }
        HistoryEntryKind::AddTrustedApprover => "Добавлено доверенное лицо".to_string(),
        HistoryEntryKind::RequestWithdrawal { amount, .. } => {
            format!("Запрос на перевод {}", money(*amount, decimals))
        }
        HistoryEntryKind::ApproveWithdrawal => "Перевод одобрен".to_string(),
        HistoryEntryKind::InitializeVault => "Счёт открыт".to_string(),
        HistoryEntryKind::InitializeApprovers => "Включены доверенные лица".to_string(),
        HistoryEntryKind::Unknown => "Операция".to_string(),
    }
}

/// The Anchor error code inside a node's error text, e.g.
/// "custom program error: 0x1775" -> 6005.
fn custom_error_code(text: &str) -> Option<u32> {
    let marker = "custom program error: 0x";
    let start = text.find(marker)? + marker.len();
    let hex: String = text[start..]
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .collect();
    u32::from_str_radix(&hex, 16).ok()
}

/// A sentence for a rejection we can explain, or `None`.
pub fn friendly_rejection(text: &str) -> Option<&'static str> {
    let code = custom_error_code(text)?;
    let is = |e: VaultError| code == u32::from(e);
    Some(if is(VaultError::BeneficiaryCooldownActive) {
        "Этого получателя ещё нельзя использовать: после добавления нужно подождать 48 часов."
    } else if is(VaultError::UnknownBeneficiary) {
        "Этого человека нет в списке получателей."
    } else if is(VaultError::DailyLimitExceeded) {
        "Это больше, чем можно отправить за день. Нужен запрос на перевод."
    } else if is(VaultError::AmountDoesNotExceedDailyLimit) {
        "Эту сумму можно отправить сразу, без запроса."
    } else if is(VaultError::NotVaultOwner) {
        "Это может сделать только владелец счёта."
    } else if is(VaultError::NotTrustedApprover) {
        "Одобрять переводы могут только доверенные лица."
    } else if is(VaultError::BeneficiaryListFull) {
        "В списке получателей больше нет места."
    } else if is(VaultError::BeneficiaryAlreadyExists) {
        "Этот получатель уже есть в списке."
    } else if is(VaultError::ApproverListFull) {
        "В списке доверенных лиц больше нет места."
    } else if is(VaultError::ApproverAlreadyExists) {
        "Это доверенное лицо уже добавлено."
    } else if is(VaultError::CannotApproveOwnWithdrawals) {
        "Владелец счёта не может быть своим же доверенным лицом."
    } else if is(VaultError::ExistingApproverRequired) {
        "Чтобы добавить ещё одно доверенное лицо, нужна подпись уже добавленного."
    } else if is(VaultError::RequestAlreadyExecuted) {
        "Этот запрос уже выполнен."
    } else if is(VaultError::ZeroAmount) {
        "Сумма должна быть больше нуля."
    } else {
        return None;
    })
}

pub fn error_dto(error: &ActionError) -> ErrorDto {
    match error {
        ActionError::NoConnection => ErrorDto::new(
            "no_connection",
            "Нет связи. Проверьте интернет и попробуйте ещё раз.",
        ),
        ActionError::Rejected(text) => {
            let message = friendly_rejection(text).unwrap_or("Не получилось. Ничего не списано.");
            ErrorDto::new("rejected", message).with_details(text.clone())
        }
        ActionError::Unconfirmed { signature, .. } => ErrorDto::new(
            "unconfirmed",
            "Операция отправлена, но мы пока не знаем, дошла ли она. \
             Не повторяйте её: нажмите «Проверить».",
        )
        .with_details(signature.to_string()),
    }
}

/// What to tell the user once an unconfirmed operation has been looked up.
/// `Confirmed`, `Failed` and `Expired` settle it (the record may be deleted);
/// `StillPossible` does not.
pub fn resolution_for(outcome: &SignatureOutcome) -> ResolutionDto {
    match outcome {
        SignatureOutcome::Confirmed => ResolutionDto {
            status: "confirmed",
            message: "Операция прошла.".to_string(),
        },
        SignatureOutcome::Failed(_) => ResolutionDto {
            status: "failed",
            message: "Операция не выполнилась, деньги не списаны. Можно попробовать ещё раз."
                .to_string(),
        },
        SignatureOutcome::Expired => ResolutionDto {
            status: "expired",
            message: "Операция так и не дошла, деньги не списаны. Можно попробовать ещё раз."
                .to_string(),
        },
        SignatureOutcome::StillPossible => ResolutionDto {
            status: "waiting",
            message: "Пока неизвестно, дошла ли операция. Подождите минуту и проверьте ещё раз."
                .to_string(),
        },
    }
}

pub fn pairing_error_dto(error: &PairingError) -> ErrorDto {
    let (kind, message): (&'static str, &str) = match error {
        PairingError::BadQrText => (
            "invalid",
            "Это не похоже на код с экрана устройства. Наведите камеру на QR-код ещё раз.",
        ),
        PairingError::BadPin => ("invalid", "Код состоит ровно из 6 цифр."),
        PairingError::BadName(_) => (
            "invalid",
            "Имя нужно написать короче и без служебных символов.",
        ),
        PairingError::NoAdapter => (
            "rejected",
            "Не найден Bluetooth. Включите Bluetooth и попробуйте ещё раз.",
        ),
        PairingError::KioskNotFound => (
            "rejected",
            "Устройство не найдено. Подойдите к нему ближе и убедитесь, что на экране показан QR-код.",
        ),
        PairingError::ConnectFailed(_) => (
            "rejected",
            "Не удалось соединиться с устройством. Подойдите ближе и попробуйте ещё раз.",
        ),
        PairingError::WriteRejected(_) => (
            "rejected",
            "Устройство не приняло код. Проверьте код: на экране мог появиться новый.",
        ),
        PairingError::Ble(_) => ("rejected", "Ошибка Bluetooth. Попробуйте ещё раз."),
    };
    let details = match error {
        PairingError::BadName(d)
        | PairingError::ConnectFailed(d)
        | PairingError::WriteRejected(d)
        | PairingError::Ble(d) => Some(d.clone()),
        _ => None,
    };
    let dto = ErrorDto::new(kind, message);
    match details {
        Some(d) => dto.with_details(d),
        None => dto,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> Pubkey {
        Pubkey::new_from_array([byte; 32])
    }

    #[test]
    fn plural_ru_follows_the_russian_rules() {
        let h = |n| plural_ru(n, "час", "часа", "часов");
        assert_eq!(h(1), "1 час");
        assert_eq!(h(2), "2 часа");
        assert_eq!(h(4), "4 часа");
        assert_eq!(h(5), "5 часов");
        assert_eq!(h(11), "11 часов");
        assert_eq!(h(12), "12 часов");
        assert_eq!(h(14), "14 часов");
        assert_eq!(h(21), "21 час");
        assert_eq!(h(22), "22 часа");
        assert_eq!(h(48), "48 часов");
        assert_eq!(h(111), "111 часов");
    }

    #[test]
    fn short_address_keeps_the_first_and_last_four() {
        let text = short_address(&key(7));
        let full = key(7).to_string();
        assert!(text.starts_with(&full[..4]));
        assert!(text.ends_with(&full[full.len() - 4..]));
        assert!(text.contains('…'));
    }

    #[test]
    fn money_formats_with_the_token_decimals() {
        assert_eq!(money(2_500_000, 6), "2.5 USDC");
        assert_eq!(money(0, 6), "0 USDC");
    }

    #[test]
    fn payee_status_counts_hours_until_the_payee_can_be_used() {
        let now = 1_000_000;
        assert_eq!(payee_status(now - 1, now), "Можно отправлять");
        assert_eq!(payee_status(now, now), "Можно отправлять");
        assert_eq!(
            payee_status(now + 60, now),
            "Можно будет меньше чем через час"
        );
        assert_eq!(payee_status(now + 3600, now), "Можно будет через 1 час");
        assert_eq!(payee_status(now + 3601, now), "Можно будет через 2 часа");
        assert_eq!(
            payee_status(now + 48 * 3600, now),
            "Можно будет через 48 часов"
        );
    }

    #[test]
    fn payee_status_survives_an_absurd_clock() {
        assert_eq!(
            payee_status(i64::MAX, i64::MIN),
            format!(
                "Можно будет через {}",
                plural_ru((i64::MAX as u64).div_ceil(3600), "час", "часа", "часов")
            )
        );
    }

    #[test]
    fn history_lines_are_plain_language_and_use_the_decimals() {
        assert_eq!(
            history_line(&HistoryEntryKind::Deposit { amount: 12_340_000 }, 6),
            "Пополнение на 12.34 USDC"
        );
        assert_eq!(
            history_line(&HistoryEntryKind::Withdraw { amount: 1_000_000 }, 6),
            "Отправлено 1 USDC"
        );
        assert_eq!(
            history_line(&HistoryEntryKind::InitializeVault, 6),
            "Счёт открыт"
        );
    }

    #[test]
    fn history_lines_clean_names_that_came_from_the_chain() {
        let line = history_line(
            &HistoryEntryKind::AddBeneficiary {
                name: "Саша\nПЕРЕВЕДИ\u{202E}".to_string(),
            },
            6,
        );
        assert!(!line.contains('\n'));
        assert!(!line.contains('\u{202E}'));
        assert!(line.starts_with("Добавлен получатель «Саша"));
    }

    #[test]
    fn a_custom_program_error_code_is_read_from_the_node_text() {
        let text = "RPC response error -32002: Transaction simulation failed: \
                    Error processing Instruction 0: custom program error: 0x1775; ...";
        assert_eq!(custom_error_code(text), Some(0x1775));
        assert_eq!(custom_error_code("nothing here"), None);
        assert_eq!(custom_error_code("custom program error: 0xZZ"), None);
    }

    #[test]
    fn known_program_errors_get_a_sentence_and_unknown_ones_do_not() {
        let code = u32::from(VaultError::BeneficiaryCooldownActive);
        let text = format!("custom program error: {code:#x}");
        assert!(friendly_rejection(&text).unwrap().contains("48 часов"));
        assert_eq!(friendly_rejection("custom program error: 0x1"), None);
        assert_eq!(friendly_rejection("blockhash not found"), None);
    }

    #[test]
    fn every_error_kind_has_its_own_screen_kind() {
        assert_eq!(error_dto(&ActionError::NoConnection).kind, "no_connection");
        let rejected = error_dto(&ActionError::Rejected("boom".into()));
        assert_eq!(rejected.kind, "rejected");
        assert_eq!(rejected.details.as_deref(), Some("boom"));
        let unconfirmed = error_dto(&ActionError::Unconfirmed {
            signature: solana_signature::Signature::from([1u8; 64]),
            last_valid_block_height: 9,
        });
        assert_eq!(unconfirmed.kind, "unconfirmed");
        assert!(unconfirmed.message.contains("Не повторяйте"));
    }

    #[test]
    fn only_a_still_possible_operation_stays_unsettled() {
        let status = |o: SignatureOutcome| resolution_for(&o).status;
        assert_eq!(status(SignatureOutcome::Confirmed), "confirmed");
        assert_eq!(status(SignatureOutcome::Failed("x".into())), "failed");
        assert_eq!(status(SignatureOutcome::Expired), "expired");
        assert_eq!(status(SignatureOutcome::StillPossible), "waiting");
        assert!(resolution_for(&SignatureOutcome::Failed("x".into()))
            .message
            .contains("не списаны"));
    }

    #[test]
    fn pairing_errors_are_explained_without_jargon() {
        assert_eq!(
            pairing_error_dto(&PairingError::BadPin).message,
            "Код состоит ровно из 6 цифр."
        );
        let text = pairing_error_dto(&PairingError::KioskNotFound).message;
        assert!(text.contains("ближе"));
        let rejected = pairing_error_dto(&PairingError::WriteRejected("att 0x0e".into()));
        assert_eq!(rejected.details.as_deref(), Some("att 0x0e"));
    }
}
