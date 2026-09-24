// Slint-generated code requires unsafe; allow it only in this module
mod slint_ui {
    #![allow(unsafe_code)]
    slint::include_modules!();
}

use slint_ui::*;

mod chain;
mod setup;
#[allow(dead_code)] // used starting in Task 6
mod pairing;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use slint::Model;

use chain::{ChainConfig, ChainError, VaultSnapshot};

fn config_path() -> std::path::PathBuf {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--config" {
            if let Some(path) = args.next() {
                return std::path::PathBuf::from(path);
            }
        }
    }
    let default = std::path::PathBuf::from("/etc/family-vault/kiosk.toml");
    if default.exists() {
        return default;
    }
    std::path::PathBuf::from("kiosk.toml")
}

fn error_message_text(err: &ChainError) -> String {
    match err {
        ChainError::NoConnection => "Нет связи. Попробуй ещё раз позже".to_string(),
        ChainError::ProgramRejected(raw) => {
            eprintln!("[kiosk-gui] transaction failed: {raw}");
            "Не получилось. Попробуй ещё раз или позови родственника".to_string()
        }
    }
}

fn apply_snapshot(app: &AppWindow, is_pool: bool, snapshot: VaultSnapshot) {
    let items: Vec<BeneficiaryItem> = snapshot
        .beneficiaries
        .iter()
        .map(|(id, name)| BeneficiaryItem {
            name: name.as_str().into(),
            id: *id as i32,
        })
        .collect();
    let model = Rc::new(slint::VecModel::from(items));
    if is_pool {
        app.set_family_balance_text(snapshot.balance_text.as_str().into());
        app.set_family_beneficiaries(model.into());
    } else {
        app.set_balance_text(snapshot.balance_text.as_str().into());
        app.set_beneficiaries(model.into());
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;
    let config_path = config_path();

    let cfg: Arc<Mutex<Option<Arc<ChainConfig>>>> = Arc::new(Mutex::new(None));

    if setup::needs_setup(&config_path) {
        let cfg_slot = cfg.clone();
        run_setup_flow(app.as_weak(), config_path.clone(), cfg_slot);
    } else {
        match chain::load_config(&config_path) {
            Ok(loaded) => {
                *cfg.lock().unwrap() = Some(Arc::new(loaded));
            }
            Err(e) => {
                eprintln!("[kiosk-gui] failed to load config: {e}");
                app.set_balance_text("Нет связи. Попробуй ещё раз позже".into());
                app.set_family_balance_text("Нет связи. Попробуй ещё раз позже".into());
                app.run()?;
                return Ok(());
            }
        }
    }

    app.set_presets(
        Rc::new(slint::VecModel::from(vec![
            "10 USDC".into(),
            "20 USDC".into(),
            "50 USDC".into(),
        ]))
        .into(),
    );

    // Last-fetched snapshots, kept around so a beneficiary tap (which only
    // carries a small integer `id`) can be resolved back to the wallet
    // pubkey `submit_withdraw` needs. `chain.rs` doesn't track this itself
    // — see Task 4's Interfaces note.
    let personal_beneficiaries: Arc<Mutex<Vec<(u8, solana_pubkey::Pubkey)>>> =
        Arc::new(Mutex::new(Vec::new()));
    let pool_beneficiaries: Arc<Mutex<Vec<(u8, solana_pubkey::Pubkey)>>> =
        Arc::new(Mutex::new(Vec::new()));

    // Initial fetch, on a background thread so the UI shows immediately.
    refresh_vault_state_if_ready(
        app.as_weak(),
        &cfg,
        personal_beneficiaries.clone(),
        pool_beneficiaries.clone(),
    );

    // A family member using a separate family app can add a beneficiary
    // (or change a balance) at any time, independent of this device — the
    // on-chain BeneficiaryList/Vault accounts are the only source of truth
    // kiosk-gui has, there's no push notification from chain to this
    // device. Re-fetch periodically so a newly-added payee's button shows
    // up here without requiring the app to be restarted. 30s is frequent
    // enough that "just added, why isn't it here" doesn't happen, and
    // cheap enough (a couple of RPC reads) not to matter for a device that
    // is otherwise idle almost all the time.
    let refresh_timer = slint::Timer::default();
    {
        let app_weak = app.as_weak();
        let cfg = cfg.clone();
        let personal_beneficiaries = personal_beneficiaries.clone();
        let pool_beneficiaries = pool_beneficiaries.clone();
        refresh_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_secs(30),
            move || {
                refresh_vault_state_if_ready(
                    app_weak.clone(),
                    &cfg,
                    personal_beneficiaries.clone(),
                    pool_beneficiaries.clone(),
                );
            },
        );
    }

    // Currently-selected My Wallet beneficiary id, carried through the
    // AmountSelect -> Confirm screen transition (which only round-trips
    // the display name via selected-recipient-name/confirm-recipient-name)
    // so `on_send_confirmed` can resolve the wallet address by id rather
    // than by re-matching on the (not-guaranteed-unique) display name.
    let selected_beneficiary_id: Rc<RefCell<i32>> = Rc::new(RefCell::new(0));
    {
        let app_weak = app.as_weak();
        let selected_beneficiary_id = selected_beneficiary_id.clone();
        app.on_beneficiary_selected(move |id| {
            let app = app_weak.unwrap();
            let model = app.get_beneficiaries();
            let mut found_name: slint::SharedString = "Получатель".into();
            for i in 0..model.row_count() {
                if let Some(item) = model.row_data(i) {
                    if item.id == id {
                        found_name = item.name;
                        break;
                    }
                }
            }
            *selected_beneficiary_id.borrow_mut() = id;
            app.set_selected_recipient_name(found_name);
            app.set_current_screen(Screen::AmountSelect);
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_back_requested(move || {
            let app = app_weak.unwrap();
            app.set_current_screen(Screen::MyWallet);
        });
    }

    let selected_amount: Rc<RefCell<u64>> = Rc::new(RefCell::new(0));
    {
        let app_weak = app.as_weak();
        let selected_amount = selected_amount.clone();
        app.on_amount_selected(move |index| {
            let app = app_weak.unwrap();
            let presets: [u64; 3] = [10, 20, 50];
            let amount = presets.get(index as usize).copied().unwrap_or(0);
            *selected_amount.borrow_mut() = amount;
            app.set_confirm_recipient_name(app.get_selected_recipient_name());
            app.set_confirm_amount_text(chain::format_amount(amount).into());
            app.set_current_screen(Screen::Confirm);
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_send_cancelled(move || {
            let app = app_weak.unwrap();
            app.set_current_screen(Screen::MyWallet);
        });
    }
    {
        let app_weak = app.as_weak();
        let cfg = cfg.clone();
        let personal_beneficiaries = personal_beneficiaries.clone();
        let selected_amount = selected_amount.clone();
        let selected_beneficiary_id = selected_beneficiary_id.clone();
        app.on_send_confirmed(move || {
            let app = app_weak.unwrap();

            let Some(cfg) = cfg.lock().unwrap().clone() else {
                app.set_result_success(false);
                app.set_result_message_text(
                    "Не получилось. Попробуй ещё раз или позови родственника".into(),
                );
                app.set_current_screen(Screen::Result);
                return;
            };

            let recipient_name = app.get_confirm_recipient_name();

            // Resolve the wallet pubkey for the beneficiary the user
            // actually tapped, using the id stored by on_beneficiary_selected
            // rather than re-matching on the display name (names are not
            // guaranteed unique on-chain, so name-matching could resolve to
            // the wrong beneficiary).
            let recipient_id = *selected_beneficiary_id.borrow();
            let addr = personal_beneficiaries
                .lock()
                .unwrap()
                .iter()
                .find(|(id, _)| *id as i32 == recipient_id)
                .map(|(_, addr)| *addr);

            let Some(addr) = addr else {
                app.set_result_success(false);
                app.set_result_message_text(
                    "Не получилось. Попробуй ещё раз или позови родственника".into(),
                );
                app.set_current_screen(Screen::Result);
                return;
            };

            let amount = *selected_amount.borrow();
            let app_weak = app.as_weak();
            std::thread::spawn(move || {
                let result = chain::submit_withdraw(&cfg, false, addr, amount);
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(app) = app_weak.upgrade() else {
                        return;
                    };
                    match result {
                        Ok(()) => {
                            app.set_result_success(true);
                            app.set_result_message_text(
                                format!("Готово! Деньги отправлены {recipient_name}").into(),
                            );
                        }
                        Err(e) => {
                            app.set_result_success(false);
                            app.set_result_message_text(error_message_text(&e).into());
                        }
                    }
                    app.set_current_screen(Screen::Result);
                });
            });
        });
    }
    {
        let app_weak = app.as_weak();
        let cfg = cfg.clone();
        let personal_beneficiaries = personal_beneficiaries.clone();
        let pool_beneficiaries = pool_beneficiaries.clone();
        app.on_ok_pressed(move || {
            let app = app_weak.unwrap();
            app.set_current_screen(Screen::MyWallet);
            // A send just happened — refresh right away rather than
            // waiting for the 30s timer, so the balance the user sees
            // immediately reflects it (and picks up anything a family
            // member changed in the meantime too).
            refresh_vault_state_if_ready(
                app.as_weak(),
                &cfg,
                personal_beneficiaries.clone(),
                pool_beneficiaries.clone(),
            );
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_family_wallet_requested(move || {
            let app = app_weak.unwrap();
            app.set_current_screen(Screen::FamilyWallet);
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_back_to_my_wallet_requested(move || {
            let app = app_weak.unwrap();
            app.set_current_screen(Screen::MyWallet);
        });
    }
    {
        let app_weak = app.as_weak();
        let cfg = cfg.clone();
        let pool_beneficiaries = pool_beneficiaries.clone();
        app.on_family_beneficiary_selected(move |id| {
            let app = app_weak.unwrap();

            let Some(cfg) = cfg.lock().unwrap().clone() else {
                app.set_result_success(false);
                app.set_result_message_text(
                    "Не получилось. Попробуй ещё раз или позови родственника".into(),
                );
                app.set_current_screen(Screen::Result);
                return;
            };

            let model = app.get_family_beneficiaries();
            let mut found_name: slint::SharedString = "Получатель".into();
            for i in 0..model.row_count() {
                if let Some(item) = model.row_data(i) {
                    if item.id == id {
                        found_name = item.name;
                        break;
                    }
                }
            }
            let addr = pool_beneficiaries
                .lock()
                .unwrap()
                .iter()
                .find(|(bid, _)| *bid as i32 == id)
                .map(|(_, addr)| *addr);
            let Some(addr) = addr else { return };

            // Family Wallet has no dedicated amount-select/confirm screens
            // of its own in this plan's scope (spec: "Family Wallet" data
            // flow reuses the same withdraw call) — send the first preset
            // amount directly and reuse the shared Result screen. A richer
            // Family Wallet amount-select flow is future work, not part of
            // this plan.
            let amount = 10u64;
            let cfg = cfg.clone();
            let app_weak = app.as_weak();
            std::thread::spawn(move || {
                let result = chain::submit_withdraw(&cfg, true, addr, amount);
                let _ = slint::invoke_from_event_loop(move || {
                    let Some(app) = app_weak.upgrade() else {
                        return;
                    };
                    match result {
                        Ok(()) => {
                            app.set_result_success(true);
                            app.set_result_message_text(
                                format!("Готово! Деньги отправлены {found_name}").into(),
                            );
                        }
                        Err(e) => {
                            app.set_result_success(false);
                            app.set_result_message_text(error_message_text(&e).into());
                        }
                    }
                    app.set_current_screen(Screen::Result);
                });
            });
        });
    }
    app.on_help_requested(|| {
        println!("help requested");
    });
    app.on_family_help_requested(|| {
        println!("help requested (family wallet)");
    });

    app.run()
}

/// Drives the entire first-run setup flow on a background thread: load
/// provisioning config, generate-or-load the owner keypair, show its QR,
/// poll for funding, self-initialize the vault, write kiosk.toml, then
/// switch to the normal My Wallet flow — all without restarting the
/// process. See the design spec's "Фандинг и самостоятельная
/// инициализация" section for the exact sequence this implements.
fn run_setup_flow(
    app_weak: slint::Weak<AppWindow>,
    config_path: std::path::PathBuf,
    cfg_slot: Arc<Mutex<Option<Arc<ChainConfig>>>>,
) {
    let app_weak_for_screen = app_weak.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(app) = app_weak_for_screen.upgrade() {
            app.set_current_screen(Screen::Setup);
        }
    });

    std::thread::spawn(move || {
        let config_dir = config_path
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let provisioning_path = config_dir.join("provisioning.toml");

        let provisioning = match setup::load_provisioning_config(&provisioning_path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[kiosk-gui] setup: failed to load provisioning.toml: {e}");
                set_setup_status(
                    &app_weak,
                    "Не получилось. Обратитесь в поддержку".to_string(),
                );
                return;
            }
        };

        let keypair_path = setup::owner_keypair_path(&config_dir);
        let owner = match setup::generate_or_load_owner_keypair(&keypair_path) {
            Ok(k) => k,
            Err(e) => {
                eprintln!("[kiosk-gui] setup: failed to generate/load keypair: {e}");
                set_setup_status(
                    &app_weak,
                    "Не получилось. Обратитесь в поддержку".to_string(),
                );
                return;
            }
        };

        let owner_pubkey_str = solana_signer::Signer::pubkey(&owner).to_string();
        {
            // `slint::Image` is not `Send`, so it can't be built on this
            // background thread and moved into the UI-thread closure —
            // render it inside the closure instead, from the (Send)
            // pubkey string.
            let app_weak = app_weak.clone();
            let owner_pubkey_str = owner_pubkey_str.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(app) = app_weak.upgrade() {
                    let qr = setup::render_qr(&owner_pubkey_str);
                    app.set_setup_qr_image(qr);
                    app.set_setup_status_text(
                        "Покажите этот код родственнику, у которого будет доступ к приложению семьи"
                            .into(),
                    );
                }
            });
        }

        let rpc = solana_client::rpc_client::RpcClient::new(provisioning.rpc_url.clone());
        let owner_pubkey_client = solana_signer::Signer::pubkey(&owner);

        let min_lamports = loop {
            match setup::compute_min_funding_lamports(&rpc) {
                Ok(v) => break v,
                Err(_) => std::thread::sleep(std::time::Duration::from_secs(4)),
            }
        };

        set_setup_status(&app_weak, "Ожидаем подтверждение...".to_string());

        loop {
            match rpc.get_balance(&owner_pubkey_client) {
                Ok(balance) if balance >= min_lamports => break,
                _ => std::thread::sleep(std::time::Duration::from_secs(4)),
            }
        }

        set_setup_status(&app_weak, "Почти готово...".to_string());

        let mut vault_result = None;
        for attempt in 0..5u32 {
            match setup::self_initialize_personal_vault(&rpc, &owner, &provisioning) {
                Ok(v) => {
                    vault_result = Some(v);
                    break;
                }
                Err(e) => {
                    eprintln!(
                        "[kiosk-gui] setup: initialize_vault attempt {attempt} failed: {e:?}"
                    );
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            }
        }

        let Some((vault, beneficiary_list)) = vault_result else {
            set_setup_status(
                &app_weak,
                "Не получилось. Обратитесь в поддержку".to_string(),
            );
            return;
        };

        let values = setup::KioskConfigValues {
            rpc_url: provisioning.rpc_url.clone(),
            keypair_path: keypair_path.to_string_lossy().into_owned(),
            program_id: provisioning.program_id.to_string(),
            mint: provisioning.mint.to_string(),
            personal_vault: vault.to_string(),
            personal_beneficiary_list: beneficiary_list.to_string(),
        };
        if let Err(e) = setup::write_kiosk_config(&config_path, &values) {
            eprintln!("[kiosk-gui] setup: failed to write kiosk.toml: {e}");
            set_setup_status(
                &app_weak,
                "Не получилось. Обратитесь в поддержку".to_string(),
            );
            return;
        }

        // Load the config we just wrote (exercises the exact same code
        // path a normal boot uses) and switch straight into the normal
        // flow in this same process — no restart.
        let cfg = match chain::load_config(&config_path) {
            Ok(cfg) => std::sync::Arc::new(cfg),
            Err(e) => {
                eprintln!("[kiosk-gui] setup: wrote kiosk.toml but failed to reload it: {e}");
                set_setup_status(
                    &app_weak,
                    "Не получилось. Обратитесь в поддержку".to_string(),
                );
                return;
            }
        };

        *cfg_slot.lock().unwrap() = Some(cfg.clone());

        let personal_beneficiaries: std::sync::Arc<
            std::sync::Mutex<Vec<(u8, solana_pubkey::Pubkey)>>,
        > = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let pool_beneficiaries: std::sync::Arc<
            std::sync::Mutex<Vec<(u8, solana_pubkey::Pubkey)>>,
        > = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

        let app_weak_for_transition = app_weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(app) = app_weak_for_transition.upgrade() {
                app.set_current_screen(Screen::MyWallet);
            }
        });
        refresh_vault_state(app_weak, cfg, personal_beneficiaries, pool_beneficiaries);
    });
}

fn set_setup_status(app_weak: &slint::Weak<AppWindow>, text: String) {
    let app_weak = app_weak.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(app) = app_weak.upgrade() {
            app.set_setup_status_text(text.into());
        }
    });
}

/// Wraps `refresh_vault_state` so it becomes a no-op when `cfg` hasn't
/// been populated yet — i.e. setup mode is still in progress. Used at
/// every call site in `main()` instead of calling `refresh_vault_state`
/// directly, since `cfg` may not exist yet on a fresh first boot.
fn refresh_vault_state_if_ready(
    app_weak: slint::Weak<AppWindow>,
    cfg: &Arc<Mutex<Option<Arc<ChainConfig>>>>,
    personal_beneficiaries: Arc<Mutex<Vec<(u8, solana_pubkey::Pubkey)>>>,
    pool_beneficiaries: Arc<Mutex<Vec<(u8, solana_pubkey::Pubkey)>>>,
) {
    let Some(cfg) = cfg.lock().unwrap().clone() else {
        return;
    };
    refresh_vault_state(app_weak, cfg, personal_beneficiaries, pool_beneficiaries);
}

/// Fetches both vaults' current balance/beneficiary state on a background
/// thread and applies it to the UI once done. Used both for the initial
/// load and the periodic refresh (see the timer in `main`) — a family
/// member's separate app can change on-chain state at any time, and this
/// is how that eventually becomes visible here.
fn refresh_vault_state(
    app_weak: slint::Weak<AppWindow>,
    cfg: Arc<ChainConfig>,
    personal_beneficiaries: Arc<Mutex<Vec<(u8, solana_pubkey::Pubkey)>>>,
    pool_beneficiaries: Arc<Mutex<Vec<(u8, solana_pubkey::Pubkey)>>>,
) {
    std::thread::spawn(move || {
        let personal_result = chain::fetch_vault_snapshot(&cfg, false);
        let pool_result = chain::fetch_vault_snapshot(&cfg, true);

        // Re-derive wallet pubkeys for beneficiary-id resolution. The
        // snapshot only carries (id, name) for display, so beneficiary
        // list bytes are re-fetched here to also capture (id, address)
        // for later withdraw calls.
        let personal_addrs = fetch_beneficiary_addresses(&cfg, false);
        let pool_addrs = fetch_beneficiary_addresses(&cfg, true);

        let _ = slint::invoke_from_event_loop(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            match personal_result {
                Ok(snapshot) => apply_snapshot(&app, false, snapshot),
                Err(e) => app.set_balance_text(error_message_text(&e).into()),
            }
            match pool_result {
                Ok(snapshot) => apply_snapshot(&app, true, snapshot),
                Err(e) => app.set_family_balance_text(error_message_text(&e).into()),
            }
            if let Ok(addrs) = personal_addrs {
                *personal_beneficiaries.lock().unwrap() = addrs;
            }
            if let Ok(addrs) = pool_addrs {
                *pool_beneficiaries.lock().unwrap() = addrs;
            }
        });
    });
}

fn fetch_beneficiary_addresses(
    cfg: &ChainConfig,
    is_pool: bool,
) -> Result<Vec<(u8, solana_pubkey::Pubkey)>, ChainError> {
    use anchor_lang::AccountDeserialize;

    let rpc = solana_client::rpc_client::RpcClient::new(cfg.rpc_url.clone());
    let list_pubkey = if is_pool {
        cfg.pool_beneficiary_list
    } else {
        cfg.personal_beneficiary_list
    };
    let bytes = rpc
        .get_account_data(&list_pubkey)
        .map_err(chain::map_client_error_pub)?;
    let list = family_vault::state::BeneficiaryList::try_deserialize(&mut &bytes[..])
        .map_err(|e| ChainError::ProgramRejected(e.to_string()))?;
    Ok((0..list.count as usize)
        .map(|i| {
            let b = &list.entries[i];
            (
                (i + 1) as u8,
                solana_pubkey::Pubkey::new_from_array(b.address.to_bytes()),
            )
        })
        .collect())
}
