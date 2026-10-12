//! What the app says, as catalog keys with parameters (`Msg`), and the pure
//! functions that turn core values into them. The sentences themselves live in
//! `ui/locales/*.js`, in plain bank language: no "vault", "PDA", "mint" or
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
use crate::msg::Msg;

pub fn short_address(address: &Pubkey) -> String {
    let full = address.to_string();
    format!("{}…{}", &full[..4], &full[full.len() - 4..])
}

pub fn money(amount: u64, decimals: u32) -> String {
    format!("{} USDC", format_base_units(amount, decimals))
}

pub fn payee_status(active_after: i64, now: i64) -> Msg {
    if now >= active_after {
        return Msg::new("payee.ready");
    }
    let secs = active_after.saturating_sub(now) as u64;
    if secs < 3600 {
        return Msg::new("payee.wait_under_hour");
    }
    Msg::new("payee.wait_hours").with("n", secs.div_ceil(3600))
}

pub fn history_line(kind: &HistoryEntryKind, decimals: u32) -> Msg {
    // Names in history come straight from the chain: clean them first.
    let name = |name: &str| sanitize_text(name);
    match kind {
        HistoryEntryKind::Deposit { amount } => {
            Msg::new("history.deposit").with("amount", money(*amount, decimals))
        }
        HistoryEntryKind::Withdraw { amount } => {
            Msg::new("history.withdraw").with("amount", money(*amount, decimals))
        }
        HistoryEntryKind::AddBeneficiary { name: who } => {
            Msg::new("history.payee_added").with("name", name(who))
        }
        HistoryEntryKind::AddBeneficiaryUrgent { name: who } => {
            Msg::new("history.payee_added_urgent").with("name", name(who))
        }
        HistoryEntryKind::AddTrustedApprover => Msg::new("history.guardian_added"),
        HistoryEntryKind::RequestWithdrawal { amount, .. } => {
            Msg::new("history.request").with("amount", money(*amount, decimals))
        }
        HistoryEntryKind::ApproveWithdrawal => Msg::new("history.approved"),
        HistoryEntryKind::InitializeVault => Msg::new("history.opened"),
        HistoryEntryKind::InitializeApprovers => Msg::new("history.guardians_enabled"),
        HistoryEntryKind::Unknown => Msg::new("history.unknown"),
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

/// The catalog key of a rejection we can explain, or `None`.
pub fn friendly_rejection(text: &str) -> Option<&'static str> {
    let code = custom_error_code(text)?;
    let is = |e: VaultError| code == u32::from(e);
    Some(if is(VaultError::BeneficiaryCooldownActive) {
        "error.vault.beneficiary_cooldown_active"
    } else if is(VaultError::UnknownBeneficiary) {
        "error.vault.unknown_beneficiary"
    } else if is(VaultError::DailyLimitExceeded) {
        "error.vault.daily_limit_exceeded"
    } else if is(VaultError::AmountDoesNotExceedDailyLimit) {
        "error.vault.amount_does_not_exceed_daily_limit"
    } else if is(VaultError::NotVaultOwner) {
        "error.vault.not_vault_owner"
    } else if is(VaultError::NotTrustedApprover) {
        "error.vault.not_trusted_approver"
    } else if is(VaultError::BeneficiaryListFull) {
        "error.vault.beneficiary_list_full"
    } else if is(VaultError::BeneficiaryAlreadyExists) {
        "error.vault.beneficiary_already_exists"
    } else if is(VaultError::ApproverListFull) {
        "error.vault.approver_list_full"
    } else if is(VaultError::ApproverAlreadyExists) {
        "error.vault.approver_already_exists"
    } else if is(VaultError::CannotApproveOwnWithdrawals) {
        "error.vault.cannot_approve_own_withdrawals"
    } else if is(VaultError::ExistingApproverRequired) {
        "error.vault.existing_approver_required"
    } else if is(VaultError::RequestAlreadyExecuted) {
        "error.vault.request_already_executed"
    } else if is(VaultError::ZeroAmount) {
        "error.vault.zero_amount"
    } else {
        return None;
    })
}

pub fn error_dto(error: &ActionError) -> ErrorDto {
    match error {
        ActionError::NoConnection => ErrorDto::new("no_connection", "error.no_connection"),
        ActionError::Rejected(text) => {
            let key = friendly_rejection(text).unwrap_or("error.rejected_generic");
            ErrorDto::new("rejected", key).with_details(text.clone())
        }
        ActionError::Unconfirmed { signature, .. } => {
            ErrorDto::new("unconfirmed", "error.unconfirmed").with_details(signature.to_string())
        }
    }
}

/// What to tell the user once an unconfirmed operation has been looked up.
/// `Confirmed`, `Failed` and `Expired` settle it (the record may be deleted);
/// `StillPossible` does not.
pub fn resolution_for(outcome: &SignatureOutcome) -> ResolutionDto {
    let (status, key) = match outcome {
        SignatureOutcome::Confirmed => ("confirmed", "resolution.confirmed"),
        SignatureOutcome::Failed(_) => ("failed", "resolution.failed"),
        SignatureOutcome::Expired => ("expired", "resolution.expired"),
        SignatureOutcome::StillPossible => ("waiting", "resolution.waiting"),
    };
    ResolutionDto {
        status,
        message: Msg::new(key),
    }
}

pub fn pairing_error_dto(error: &PairingError) -> ErrorDto {
    let (kind, key): (&'static str, &str) = match error {
        PairingError::BadQrText => ("invalid", "error.pairing.bad_qr"),
        PairingError::BadPin => ("invalid", "error.pairing.bad_pin"),
        PairingError::BadName(_) => ("invalid", "error.pairing.bad_name"),
        PairingError::NoAdapter => ("rejected", "error.pairing.no_adapter"),
        PairingError::KioskNotFound => ("rejected", "error.pairing.kiosk_not_found"),
        PairingError::ConnectFailed(_) => ("rejected", "error.pairing.connect_failed"),
        PairingError::WriteRejected(_) => ("rejected", "error.pairing.write_rejected"),
        PairingError::Ble(_) => ("rejected", "error.pairing.ble"),
    };
    let details = match error {
        PairingError::BadName(d)
        | PairingError::ConnectFailed(d)
        | PairingError::WriteRejected(d)
        | PairingError::Ble(d) => Some(d.clone()),
        _ => None,
    };
    let dto = ErrorDto::new(kind, key);
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
        let hours = |n: u64| Msg::new("payee.wait_hours").with("n", n);
        assert_eq!(payee_status(now - 1, now), Msg::new("payee.ready"));
        assert_eq!(payee_status(now, now), Msg::new("payee.ready"));
        assert_eq!(
            payee_status(now + 60, now),
            Msg::new("payee.wait_under_hour")
        );
        assert_eq!(payee_status(now + 3600, now), hours(1));
        assert_eq!(payee_status(now + 3601, now), hours(2));
        assert_eq!(payee_status(now + 48 * 3600, now), hours(48));
    }

    #[test]
    fn payee_status_survives_an_absurd_clock() {
        assert_eq!(
            payee_status(i64::MAX, i64::MIN),
            Msg::new("payee.wait_hours").with("n", (i64::MAX as u64).div_ceil(3600))
        );
    }

    #[test]
    fn history_lines_carry_keys_and_the_formatted_amounts() {
        assert_eq!(
            history_line(&HistoryEntryKind::Deposit { amount: 12_340_000 }, 6),
            Msg::new("history.deposit").with("amount", "12.34 USDC")
        );
        assert_eq!(
            history_line(&HistoryEntryKind::Withdraw { amount: 1_000_000 }, 6),
            Msg::new("history.withdraw").with("amount", "1 USDC")
        );
        assert_eq!(
            history_line(&HistoryEntryKind::InitializeVault, 6),
            Msg::new("history.opened")
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
        assert_eq!(line.key, "history.payee_added");
        let name = line.params["name"].as_str().unwrap();
        assert!(!name.contains('\n'));
        assert!(!name.contains('\u{202E}'));
        assert!(name.starts_with("Саша"));
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
    fn known_program_errors_get_a_key_and_unknown_ones_do_not() {
        let code = u32::from(VaultError::BeneficiaryCooldownActive);
        let text = format!("custom program error: {code:#x}");
        assert_eq!(
            friendly_rejection(&text),
            Some("error.vault.beneficiary_cooldown_active")
        );
        assert_eq!(friendly_rejection("custom program error: 0x1"), None);
        assert_eq!(friendly_rejection("blockhash not found"), None);
    }

    #[test]
    fn every_error_kind_has_its_own_screen_kind() {
        assert_eq!(error_dto(&ActionError::NoConnection).kind, "no_connection");
        let rejected = error_dto(&ActionError::Rejected("boom".into()));
        assert_eq!(rejected.kind, "rejected");
        assert_eq!(rejected.message.key, "error.rejected_generic");
        assert_eq!(rejected.details.as_deref(), Some("boom"));
        let unconfirmed = error_dto(&ActionError::Unconfirmed {
            signature: solana_signature::Signature::from([1u8; 64]),
            last_valid_block_height: 9,
        });
        assert_eq!(unconfirmed.kind, "unconfirmed");
        assert_eq!(unconfirmed.message.key, "error.unconfirmed");
    }

    #[test]
    fn only_a_still_possible_operation_stays_unsettled() {
        let status = |o: SignatureOutcome| resolution_for(&o).status;
        assert_eq!(status(SignatureOutcome::Confirmed), "confirmed");
        assert_eq!(status(SignatureOutcome::Failed("x".into())), "failed");
        assert_eq!(status(SignatureOutcome::Expired), "expired");
        assert_eq!(status(SignatureOutcome::StillPossible), "waiting");
        assert_eq!(
            resolution_for(&SignatureOutcome::Failed("x".into()))
                .message
                .key,
            "resolution.failed"
        );
    }

    #[test]
    fn pairing_errors_have_their_own_keys_and_keep_technical_details() {
        assert_eq!(
            pairing_error_dto(&PairingError::BadPin).message.key,
            "error.pairing.bad_pin"
        );
        assert_eq!(
            pairing_error_dto(&PairingError::KioskNotFound).message.key,
            "error.pairing.kiosk_not_found"
        );
        let rejected = pairing_error_dto(&PairingError::WriteRejected("att 0x0e".into()));
        assert_eq!(rejected.details.as_deref(), Some("att 0x0e"));
    }
}
