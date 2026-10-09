//! Trellis Family: the family member's app. A thin Tauri shell around
//! `family-app-core`: `commands` is the IPC surface, `text`/`view` hold
//! every rule and sentence as pure, tested functions, and the `ui/` folder
//! only displays what these return.

mod commands;
mod config;
mod dto;
mod pending;
mod state;
mod text;
mod view;

use std::sync::Arc;

use family_app_core::identity::generate_or_load_identity;
use solana_client::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use tauri::Manager;

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
            std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
            let identity = generate_or_load_identity(&data_dir.join(config::IDENTITY_FILE))
                .map_err(|e| e.to_string())?;
            // `confirmed`, so a screen refreshed right after a send shows it.
            let rpc =
                RpcClient::new_with_commitment(config::rpc_url(), CommitmentConfig::confirmed());
            app.manage(AppState {
                identity: Arc::new(identity),
                rpc: Arc::new(rpc),
                program_id: config::program_id(),
                data_dir,
                busy: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::read::app_info,
            commands::read::wallet,
            commands::read::list_vaults,
            commands::read::load_vault,
            commands::read::load_history,
            commands::read::pending_status,
            commands::read::check_pending,
            commands::actions::deposit,
            commands::actions::send_money,
            commands::actions::add_payee,
            commands::actions::add_guardian,
            commands::actions::approve_request,
            commands::actions::open_family_account,
            commands::actions::get_test_sol,
            commands::pairing::pair_with_qr,
            commands::pairing::pair_with_pin,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Trellis Family app");
}
