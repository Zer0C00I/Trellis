//! Handing this key to a kiosk during its first-run setup (Bluetooth).
//! Both commands block for up to about a minute, so they run on a blocking
//! thread. A successful return only means the kiosk accepted the code: it
//! may still fail to register the key on the chain, so the screen must keep
//! checking `list_vaults` until a vault shows up.

use family_app_core::pairing::{pair_with_kiosk, pair_with_kiosk_pin};
use tauri::State;

use super::{acquire_busy, blocking};
use crate::dto::ErrorDto;
use crate::state::AppState;
use crate::text::pairing_error_dto;

#[tauri::command]
pub async fn pair_with_qr(
    state: State<'_, AppState>,
    qr_text: String,
    name: String,
) -> Result<(), ErrorDto> {
    let st = state.inner().clone();
    let me = st.me();
    blocking(move || {
        let _busy = acquire_busy(&st)?;
        pair_with_kiosk(&qr_text, &name, me).map_err(|e| pairing_error_dto(&e))
    })
    .await
}

#[tauri::command]
pub async fn pair_with_pin(
    state: State<'_, AppState>,
    pin: String,
    name: String,
) -> Result<(), ErrorDto> {
    let st = state.inner().clone();
    let me = st.me();
    blocking(move || {
        let _busy = acquire_busy(&st)?;
        pair_with_kiosk_pin(&pin, &name, me).map_err(|e| pairing_error_dto(&e))
    })
    .await
}
