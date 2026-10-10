//! Read-only commands: who am I, what do I have, what happened.

use std::collections::BTreeSet;

use family_app_core::actions::{confirm_signature, SignatureOutcome};
use family_app_core::discovery::{discover_my_vaults, discover_owned_vaults};
use family_app_core::history::fetch_history_cached;
use family_app_core::wallet::{sol_balance, LOW_SOL_LAMPORTS};
use family_vault_client::format_base_units;
use tauri::State;

use super::{
    acquire_busy, blocking, load_pending, load_view, now_unix, parse_pubkey,
    parse_record_signature, quarantine_error,
};
use crate::dto::{
    AppInfoDto, ErrorDto, HistoryLineDto, PendingTxDto, ResolutionDto, VaultDto, VaultRefDto,
    WalletDto,
};
use crate::pending;
use crate::state::AppState;
use crate::text::{history_line, resolution_for, short_address};
use crate::view::vault_dto;

/// How many recent transactions the history screen asks the node for.
const HISTORY_LIMIT: usize = 10;

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfoDto {
    let me = state.me();
    AppInfoDto {
        address: me.to_string(),
        address_short: short_address(&me),
        network: "devnet",
    }
}

#[tauri::command]
pub async fn wallet(state: State<'_, AppState>) -> Result<WalletDto, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        let lamports = sol_balance(&st.rpc, &st.me())?;
        Ok(WalletDto {
            sol: format_base_units(lamports, 9),
            low_sol: lamports < LOW_SOL_LAMPORTS,
        })
    })
    .await
}

/// Every vault this key owns or takes part in, the ones it owns first.
#[tauri::command]
pub async fn list_vaults(state: State<'_, AppState>) -> Result<Vec<VaultRefDto>, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        let me = st.me();
        let owned = discover_owned_vaults(&st.rpc, &st.program_id, &me)?;
        let others: BTreeSet<_> = discover_my_vaults(&st.rpc, &st.program_id, &me)?
            .into_iter()
            .filter(|v| !owned.contains(v))
            .collect();
        let to_ref = |address: solana_pubkey::Pubkey, mine: bool| VaultRefDto {
            address: address.to_string(),
            address_short: short_address(&address),
            mine,
        };
        Ok(owned
            .into_iter()
            .map(|v| to_ref(v, true))
            .chain(others.into_iter().map(|v| to_ref(v, false)))
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn load_vault(state: State<'_, AppState>, vault: String) -> Result<VaultDto, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        let view = load_view(&st, &vault)?;
        Ok(vault_dto(&view, &st.me(), now_unix()))
    })
    .await
}

/// `decimals` only formats the amounts in the lines; it comes from the
/// `VaultDto` the screen already holds.
#[tauri::command]
pub async fn load_history(
    state: State<'_, AppState>,
    vault: String,
    decimals: u32,
) -> Result<Vec<HistoryLineDto>, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        let vault = parse_pubkey(&vault, "Счёт")?;
        // A poisoned lock only means an earlier call panicked mid-update;
        // the cache is still a valid map, so keep using it.
        let mut cache = st
            .history_cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let entries =
            fetch_history_cached(&st.rpc, &st.program_id, &vault, HISTORY_LIMIT, &mut cache)?;
        drop(cache);
        Ok(entries
            .into_iter()
            .map(|e| HistoryLineDto {
                signature: e.signature,
                text: history_line(&e.kind, decimals),
                time: e.block_time,
            })
            .collect())
    })
    .await
}

/// The operation whose outcome is still unknown, if any.
#[tauri::command]
pub fn pending_status(state: State<'_, AppState>) -> Result<Option<PendingTxDto>, ErrorDto> {
    let record = load_pending(&state)?;
    Ok(record.map(|p| PendingTxDto {
        what: p.what,
        signature: p.signature,
    }))
}

/// Asks the node what became of the unconfirmed operation and, once that is
/// settled, forgets it so money commands work again.
#[tauri::command]
pub async fn check_pending(state: State<'_, AppState>) -> Result<ResolutionDto, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        // It can clear or quarantine the record, so it takes the same guard
        // as the money commands.
        let _busy = acquire_busy(&st)?;
        let record = load_pending(&st)?;
        let Some(record) = record else {
            return Ok(ResolutionDto {
                status: "none",
                message: "Ничего не ожидает проверки.".to_string(),
            });
        };
        let Some(signature) = parse_record_signature(&record) else {
            return Err(quarantine_error(
                &st,
                &format!("unparsable signature {:?}", record.signature),
            ));
        };
        let outcome = confirm_signature(&st.rpc, &signature, record.last_valid_block_height)?;
        if outcome != SignatureOutcome::StillPossible {
            pending::clear(&st.pending_path()).map_err(|e| {
                ErrorDto::new("internal", "Не удалось обновить записи приложения.").with_details(e)
            })?;
        }
        Ok(resolution_for(&outcome))
    })
    .await
}
