// Slint-generated code requires unsafe; allow it only in this module
pub mod slint_ui {
    #![allow(unsafe_code)]
    slint::include_modules!();
}

use slint_ui::*;

mod ble;
mod chain;
mod family_member;
mod language;
mod pairing;
mod setup;

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

fn error_message(err: &ChainError) -> Message {
    match err {
        ChainError::NoConnection => Message::NoConnection,
        ChainError::ProgramRejected(raw) => {
            eprintln!("[kiosk-gui] transaction failed: {raw}");
            Message::Failed
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
        app.set_family_balance_message(Message::None);
        app.set_family_beneficiaries(model.into());
    } else {
        app.set_balance_text(snapshot.balance_text.as_str().into());
        app.set_balance_message(Message::None);
        app.set_beneficiaries(model.into());
    }
}

pub fn run() -> Result<(), slint::PlatformError> {
    let config_path = config_path();
    let language =
        language::startup_language(&config_path, std::env::var("LANG").ok().as_deref());
    eprintln!("[kiosk-gui] language: {}", language.code());
    let app = AppWindow::new()?;
    if let Err(e) = slint::select_bundled_translation(language.code()) {
        eprintln!("[kiosk-gui] could not select the {} translation: {e}", language.code());
    }

    let cfg: Arc<Mutex<Option<Arc<ChainConfig>>>> = Arc::new(Mutex::new(None));

    // Set by the setup screen's skip button, read by the BLE pairing step.
    let skip_pairing = Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let skip_pairing = skip_pairing.clone();
        app.on_skip_pairing_requested(move || {
            skip_pairing.store(true, std::sync::atomic::Ordering::Relaxed);
        });
    }

    if setup::needs_setup(&config_path) {
        let cfg_slot = cfg.clone();
        run_setup_flow(
            app.as_weak(),
            config_path.clone(),
            cfg_slot,
            skip_pairing.clone(),
            language,
        );
    } else {
        match chain::load_config(&config_path) {
            Ok(loaded) => {
                *cfg.lock().unwrap() = Some(Arc::new(loaded));
            }
            Err(e) => {
                eprintln!("[kiosk-gui] failed to load config: {e}");
                app.set_balance_message(Message::NoConnection);
                app.set_family_balance_message(Message::NoConnection);
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

    // Last-fetched snapshots, to resolve a tapped beneficiary id back to its wallet pubkey.
    let personal_beneficiaries: Arc<Mutex<Vec<(u8, solana_pubkey::Pubkey)>>> =
        Arc::new(Mutex::new(Vec::new()));
    let pool_beneficiaries: Arc<Mutex<Vec<(u8, solana_pubkey::Pubkey)>>> =
        Arc::new(Mutex::new(Vec::new()));

    refresh_vault_state_if_ready(
        app.as_weak(),
        &cfg,
        personal_beneficiaries.clone(),
        pool_beneficiaries.clone(),
    );

    // Family members change on-chain state from their own app and nothing pushes it here, so re-fetch every 30s.
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

    // Selected beneficiary id, kept across AmountSelect -> Confirm: names are not unique, ids are.
    let selected_beneficiary_id: Rc<RefCell<i32>> = Rc::new(RefCell::new(0));
    {
        let app_weak = app.as_weak();
        let selected_beneficiary_id = selected_beneficiary_id.clone();
        app.on_beneficiary_selected(move |id| {
            let app = app_weak.unwrap();
            let model = app.get_beneficiaries();
            let mut found_name = slint::SharedString::default();
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
                app.set_result_message(Message::Failed);
                app.set_current_screen(Screen::Result);
                return;
            };

            let recipient_name = app.get_confirm_recipient_name();

            // Resolve by id, not by display name (names are not unique on-chain).
            let recipient_id = *selected_beneficiary_id.borrow();
            let addr = personal_beneficiaries
                .lock()
                .unwrap()
                .iter()
                .find(|(id, _)| *id as i32 == recipient_id)
                .map(|(_, addr)| *addr);

            let Some(addr) = addr else {
                app.set_result_success(false);
                app.set_result_message(Message::Failed);
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
                            app.set_result_message(Message::Done);
                            app.set_result_name(recipient_name);
                        }
                        Err(e) => {
                            app.set_result_success(false);
                            app.set_result_message(error_message(&e));
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
            // Refresh right after a send instead of waiting for the timer.
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
                app.set_result_message(Message::Failed);
                app.set_current_screen(Screen::Result);
                return;
            };

            let model = app.get_family_beneficiaries();
            let mut found_name = slint::SharedString::default();
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

            // No amount-select/confirm here: send the first preset and reuse the Result screen.
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
                            app.set_result_message(Message::Done);
                            app.set_result_name(found_name);
                        }
                        Err(e) => {
                            app.set_result_success(false);
                            app.set_result_message(error_message(&e));
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

/// First-run setup on a background thread: load config, keypair, funding QR, wait for funds,
/// initialize the vault, pair a family member, write kiosk.toml, switch to My Wallet without a restart.
fn run_setup_flow(
    app_weak: slint::Weak<AppWindow>,
    config_path: std::path::PathBuf,
    cfg_slot: Arc<Mutex<Option<Arc<ChainConfig>>>>,
    skip_pairing: Arc<std::sync::atomic::AtomicBool>,
    language: language::Language,
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
                set_setup_message(&app_weak, Message::ContactSupport);
                return;
            }
        };

        let keypair_path = setup::owner_keypair_path(&config_dir);
        let owner = match setup::generate_or_load_owner_keypair(&keypair_path) {
            Ok(k) => k,
            Err(e) => {
                eprintln!("[kiosk-gui] setup: failed to generate/load keypair: {e}");
                set_setup_message(&app_weak, Message::ContactSupport);
                return;
            }
        };

        let owner_pubkey_str = solana_signer::Signer::pubkey(&owner).to_string();
        {
            // `slint::Image` is not `Send`: build it inside the UI-thread closure from the pubkey string.
            let app_weak = app_weak.clone();
            let owner_pubkey_str = owner_pubkey_str.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(app) = app_weak.upgrade() {
                    let qr = setup::render_qr(&owner_pubkey_str);
                    app.set_setup_qr_image(qr);
                    app.set_setup_message(Message::SetupShowCode);
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

        set_setup_message(&app_weak, Message::SetupWaiting);

        loop {
            match rpc.get_balance(&owner_pubkey_client) {
                Ok(balance) if balance >= min_lamports => break,
                _ => std::thread::sleep(std::time::Duration::from_secs(4)),
            }
        }

        set_setup_message(&app_weak, Message::SetupAlmostDone);

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
            set_setup_message(&app_weak, Message::ContactSupport);
            return;
        };

        run_family_pairing_step(
            &app_weak,
            &rpc,
            &owner,
            &provisioning,
            &vault,
            &skip_pairing,
        );

        let values = setup::KioskConfigValues {
            rpc_url: provisioning.rpc_url.clone(),
            keypair_path: keypair_path.to_string_lossy().into_owned(),
            program_id: provisioning.program_id.to_string(),
            mint: provisioning.mint.to_string(),
            personal_vault: vault.to_string(),
            personal_beneficiary_list: beneficiary_list.to_string(),
            language: language.code().to_string(),
        };
        if let Err(e) = setup::write_kiosk_config(&config_path, &values) {
            eprintln!("[kiosk-gui] setup: failed to write kiosk.toml: {e}");
            set_setup_message(&app_weak, Message::ContactSupport);
            return;
        }

        let cfg = match chain::load_config(&config_path) {
            Ok(cfg) => std::sync::Arc::new(cfg),
            Err(e) => {
                eprintln!("[kiosk-gui] setup: wrote kiosk.toml but failed to reload it: {e}");
                set_setup_message(&app_weak, Message::ContactSupport);
                return;
            }
        };

        *cfg_slot.lock().unwrap() = Some(cfg.clone());

        let personal_beneficiaries: std::sync::Arc<
            std::sync::Mutex<Vec<(u8, solana_pubkey::Pubkey)>>,
        > = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let pool_beneficiaries: std::sync::Arc<std::sync::Mutex<Vec<(u8, solana_pubkey::Pubkey)>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

        let app_weak_for_transition = app_weak.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(app) = app_weak_for_transition.upgrade() {
                app.set_current_screen(Screen::MyWallet);
            }
        });
        refresh_vault_state(app_weak, cfg, personal_beneficiaries, pool_beneficiaries);
    });
}

fn set_setup_message(app_weak: &slint::Weak<AppWindow>, message: Message) {
    let app_weak = app_weak.clone();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(app) = app_weak.upgrade() {
            app.set_setup_message(message);
        }
    });
}

/// Generation of the pairing screen, see [`pairing::ViewGeneration`]. UI thread only.
static PAIRING_VIEW: pairing::ViewGeneration = pairing::ViewGeneration::new();

/// Switches the setup screen to pairing mode (QR, instruction, fallback code). Callable from any thread.
fn show_pairing_view(
    app_weak: &slint::Weak<AppWindow>,
    credentials: &pairing::PairingCredentials,
    wait: Option<std::time::Duration>,
) {
    let app_weak = app_weak.clone();
    let qr_text = credentials.qr_payload();
    let code = pairing::display_pin(credentials.pin());
    let wait_secs = pairing::wait_seconds(wait);
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(app) = app_weak.upgrade() {
            app.set_setup_qr_image(setup::render_qr(&qr_text));
            app.set_pairing_wait_secs(wait_secs);
            app.set_setup_pin_text(code.into());
            app.set_setup_pairing_active(true);
            let generation = PAIRING_VIEW.next();
            // Put the normal instruction back when the cooldown ends, unless the screen changed meanwhile.
            if let Some(wait) = wait.filter(|d| *d > std::time::Duration::ZERO) {
                slint::Timer::single_shot(wait, move || {
                    if PAIRING_VIEW.is_current(generation) {
                        if let Some(app) = app_weak.upgrade() {
                            app.set_pairing_wait_secs(0);
                        }
                    }
                });
            }
        }
    });
}

/// Leaves pairing mode and puts the funding QR back.
fn hide_pairing_view(app_weak: &slint::Weak<AppWindow>, funding_qr_text: &str) {
    let app_weak = app_weak.clone();
    let funding_qr_text = funding_qr_text.to_string();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(app) = app_weak.upgrade() {
            PAIRING_VIEW.next();
            app.set_pairing_wait_secs(0);
            app.set_setup_pairing_active(false);
            app.set_setup_pin_text("".into());
            app.set_setup_qr_image(setup::render_qr(&funding_qr_text));
        }
    });
}

/// Optional first-run step: a family member hands over their key over Bluetooth and is registered on-chain.
/// Never fatal, runs before kiosk.toml is written and is idempotent.
fn run_family_pairing_step(
    app_weak: &slint::Weak<AppWindow>,
    rpc: &solana_client::rpc_client::RpcClient,
    owner: &solana_keypair::Keypair,
    provisioning: &setup::ProvisioningConfig,
    vault: &solana_pubkey::Pubkey,
    skip_pairing: &Arc<std::sync::atomic::AtomicBool>,
) {
    let program_id = family_vault_client::to_client(provisioning.program_id);
    let (approvers_pk, _) = family_vault_client::approvers_pda(&program_id, vault);
    let (list_pk, _) = family_vault_client::beneficiary_list_pda(&program_id, vault);

    match family_member::fetch_membership_state(rpc, &approvers_pk, &list_pk) {
        Ok(state) if !family_member::needs_pairing(&state) => {
            eprintln!("[kiosk-gui] setup: a family member is already registered, skipping pairing");
            return;
        }
        Ok(_) => {}
        Err(e) => eprintln!(
            "[kiosk-gui] setup: could not read membership state ({e:?}); offering pairing anyway"
        ),
    }

    let credentials = match pairing::generate_credentials() {
        Ok(credentials) => credentials,
        Err(e) => {
            eprintln!(
                "[kiosk-gui] setup: could not generate pairing credentials ({e}); skipping pairing"
            );
            return;
        }
    };

    show_pairing_view(app_weak, &credentials, None);
    let on_rotated: ble::CredentialsRotated = {
        let app_weak = app_weak.clone();
        Arc::new(
            move |new_credentials: &pairing::PairingCredentials, cooldown: std::time::Duration| {
                let wait = (cooldown > std::time::Duration::ZERO).then_some(cooldown);
                show_pairing_view(&app_weak, new_credentials, wait)
            },
        )
    };
    let outcome = ble::wait_for_pairing(credentials, on_rotated, skip_pairing.as_ref());
    hide_pairing_view(app_weak, &solana_signer::Signer::pubkey(owner).to_string());
    set_setup_message(app_weak, Message::SetupAlmostDone);

    let request = match outcome {
        Ok(Some(request)) => request,
        Ok(None) => {
            eprintln!("[kiosk-gui] setup: family pairing skipped");
            return;
        }
        Err(e) => {
            eprintln!("[kiosk-gui] setup: {e}; skipping family pairing");
            return;
        }
    };

    set_setup_message(app_weak, Message::SetupAddingMember);
    let mut added = false;
    for attempt in 0..5u32 {
        match family_member::add_first_family_member(
            rpc,
            owner,
            &program_id,
            vault,
            &request.pubkey,
            &request.name,
        ) {
            Ok(()) => {
                eprintln!(
                    "[kiosk-gui] setup: registered family member {} ({:?})",
                    request.pubkey, request.name
                );
                added = true;
                break;
            }
            Err(e) => {
                eprintln!("[kiosk-gui] setup: add family member attempt {attempt} failed: {e:?}");
                if attempt < 4 {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                }
            }
        }
    }
    if !added {
        eprintln!(
            "[kiosk-gui] setup: could not register the family member; add them later with admin-cli"
        );
    }
    set_setup_message(app_weak, Message::SetupAlmostDone);
}

/// No-op until `cfg` exists, i.e. while setup is still running.
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

/// Fetches both vaults' balances and beneficiaries on a background thread and applies them to the UI.
fn refresh_vault_state(
    app_weak: slint::Weak<AppWindow>,
    cfg: Arc<ChainConfig>,
    personal_beneficiaries: Arc<Mutex<Vec<(u8, solana_pubkey::Pubkey)>>>,
    pool_beneficiaries: Arc<Mutex<Vec<(u8, solana_pubkey::Pubkey)>>>,
) {
    std::thread::spawn(move || {
        let personal_result = chain::fetch_vault_snapshot(&cfg, false);
        let pool_result = chain::fetch_vault_snapshot(&cfg, true);

        // The snapshot only carries (id, name); re-fetch the list to also get (id, address) for withdraws.
        let personal_addrs = fetch_beneficiary_addresses(&cfg, false);
        let pool_addrs = fetch_beneficiary_addresses(&cfg, true);

        let _ = slint::invoke_from_event_loop(move || {
            let Some(app) = app_weak.upgrade() else {
                return;
            };
            match personal_result {
                Ok(snapshot) => apply_snapshot(&app, false, snapshot),
                Err(e) => app.set_balance_message(error_message(&e)),
            }
            match pool_result {
                Ok(snapshot) => apply_snapshot(&app, true, snapshot),
                Err(e) => app.set_family_balance_message(error_message(&e)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    // The testing platform can be initialized once per process and each `#[test]` gets its own thread,
    // so every scenario runs from this single `#[test]` (same as tests/navigation.rs).
    #[test]
    fn pairing_wait_timer_uses_slint_mock_time() {
        i_slint_backend_testing::init_integration_test_with_mock_time();

        pairing_wait_notice_reverts_to_normal_after_the_cooldown_elapses();
        a_stale_timer_does_not_overwrite_a_newer_screen_state();
        the_result_screen_auto_returns_to_my_wallet_after_six_seconds();
        the_auto_return_timer_does_not_fire_while_on_a_different_screen();
    }

    /// After the cooldown the pairing timer must put the normal instruction back.
    /// Assertions run in queued closures because `invoke_from_event_loop` only queues; results come back through an `Arc<Mutex<..>>`.
    fn pairing_wait_notice_reverts_to_normal_after_the_cooldown_elapses() {
        let app = AppWindow::new().unwrap();
        let credentials = pairing::PairingCredentials::from_random_bytes([9u8; 20]);

        show_pairing_view(
            &app.as_weak(),
            &credentials,
            Some(Duration::from_secs(30)),
        );

        let captured: Arc<Mutex<Option<(i32, i32)>>> = Arc::new(Mutex::new(None));
        let captured_clone = captured.clone();
        let app_weak = app.as_weak();
        slint::invoke_from_event_loop(move || {
            let waiting = app_weak
                .upgrade()
                .map(|a| a.get_pairing_wait_secs())
                .unwrap_or_default();

            // Advance mock time past the cooldown.
            i_slint_backend_testing::mock_elapsed_time(Duration::from_secs(31));

            let after = app_weak
                .upgrade()
                .map(|a| a.get_pairing_wait_secs())
                .unwrap_or_default();
            *captured_clone.lock().unwrap() = Some((waiting, after));
            slint::quit_event_loop().unwrap();
        })
        .unwrap();

        slint::run_event_loop().unwrap();

        let (waiting, after) = captured
            .lock()
            .unwrap()
            .take()
            .expect("the queued event-loop closure ran");
        assert_eq!(waiting, 30, "the cooldown must be shown while it lasts");
        assert_eq!(after, 0, "the cooldown must be cleared once it ends");
    }

    /// A screen change before the cooldown ends must invalidate the pending timer.
    fn a_stale_timer_does_not_overwrite_a_newer_screen_state() {
        let app = AppWindow::new().unwrap();
        let credentials = pairing::PairingCredentials::from_random_bytes([9u8; 20]);

        show_pairing_view(
            &app.as_weak(),
            &credentials,
            Some(Duration::from_secs(30)),
        );

        let final_text: Arc<Mutex<Option<i32>>> = Arc::new(Mutex::new(None));
        let final_text_clone = final_text.clone();
        let app_weak = app.as_weak();
        slint::invoke_from_event_loop(move || {
            // Queue the follow-up from here so it runs after `hide_pairing_view`'s closure.
            hide_pairing_view(&app_weak, "some-funding-qr-text");

            let app_weak2 = app_weak.clone();
            let final_text_clone2 = final_text_clone.clone();
            slint::invoke_from_event_loop(move || {
                if let Some(app) = app_weak2.upgrade() {
                    app.set_pairing_wait_secs(7);
                }
                i_slint_backend_testing::mock_elapsed_time(Duration::from_secs(31));
                if let Some(app) = app_weak2.upgrade() {
                    *final_text_clone2.lock().unwrap() = Some(app.get_pairing_wait_secs());
                }
                slint::quit_event_loop().unwrap();
            })
            .unwrap();
        })
        .unwrap();

        slint::run_event_loop().unwrap();

        let final_text = final_text
            .lock()
            .unwrap()
            .take()
            .expect("the queued event-loop closure ran");
        assert_eq!(final_text, 7);
    }

    /// `app.slint`'s `auto-return-timer` must not fire before 6s and must go back to `Screen::MyWallet` after.
    /// Property access is synchronous here and `mock_elapsed_time` fires due timers without an event loop.
    fn the_result_screen_auto_returns_to_my_wallet_after_six_seconds() {
        let app = AppWindow::new().unwrap();

        app.set_current_screen(Screen::Result);

        // A zero-duration advance starts the timer's count; without it the first real advance undercounts.
        i_slint_backend_testing::mock_elapsed_time(Duration::ZERO);

        i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(5_900));
        assert_eq!(
            app.get_current_screen(),
            Screen::Result,
            "auto-return-timer fired before its 6s interval elapsed"
        );

        i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(200));
        assert_eq!(
            app.get_current_screen(),
            Screen::MyWallet,
            "auto-return-timer did not fire after its 6s interval elapsed"
        );
    }

    /// While not on the result screen the timer must not run.
    fn the_auto_return_timer_does_not_fire_while_on_a_different_screen() {
        let app = AppWindow::new().unwrap();

        // Not `Screen::MyWallet`: that is the timer's own target, so a wrong fire would go unnoticed.
        app.set_current_screen(Screen::Confirm);

        // Zero-duration advance, see the scenario above.
        i_slint_backend_testing::mock_elapsed_time(Duration::ZERO);

        i_slint_backend_testing::mock_elapsed_time(Duration::from_secs(10));
        assert_eq!(
            app.get_current_screen(),
            Screen::Confirm,
            "auto-return-timer fired while not on the Result screen"
        );
    }
}
