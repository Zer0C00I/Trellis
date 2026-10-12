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

    // Set by the setup screen's "Пропустить" button; read by the BLE
    // pairing step running on the setup thread.
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

    // Last-fetched snapshots, kept around so a beneficiary tap (which only
    // carries a small integer `id`) can be resolved back to the wallet
    // pubkey `submit_withdraw` needs. `chain.rs` doesn't track this itself.
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

            // Family Wallet has no amount-select/confirm screens of its own:
            // send the first preset amount directly and reuse the shared
            // Result screen.
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

        // Load the config we just wrote (exercises the exact same code
        // path a normal boot uses) and switch straight into the normal
        // flow in this same process — no restart.
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

/// Generation of the pairing screen; see [`pairing::ViewGeneration`]. Only
/// touched on the UI thread.
static PAIRING_VIEW: pairing::ViewGeneration = pairing::ViewGeneration::new();

/// Switches the setup screen into pairing mode: the pairing QR, the
/// instruction and the fallback code. Safe to call from any thread — the
/// `slint::Image` is built inside the UI-thread closure because it is not
/// `Send`. Called again when the kiosk rotates its credentials.
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
            // The waiting text is only true while the cooldown lasts: put
            // the normal instruction back once it ends (single change, no
            // countdown) unless the screen has changed in the meantime.
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

/// Leaves pairing mode and puts the funding QR back (which is what the
/// screen showed for the rest of setup before pairing existed).
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

/// First-run step between "personal vault exists" and "write kiosk.toml":
/// let one family member hand the kiosk their key over Bluetooth and
/// register them on-chain. Never fatal — no Bluetooth, a skip, or a chain
/// failure all just continue setup (the first family member can then be
/// added with `admin-cli`). Runs BEFORE `kiosk.toml` is written and is
/// idempotent, so a power cut anywhere in here resumes cleanly.
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
    // The funding QR is back on screen; do not leave the pairing
    // instruction next to it on the skip / BLE-unavailable paths.
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

    // `i_slint_backend_testing`'s platform can only be initialized once per
    // process (its own docs say so: the process-wide event-loop proxy is a
    // `OnceCell`), and Rust's default `#[test]` harness runs every test
    // function on its own freshly spawned OS thread -- even under
    // `--test-threads=1` -- so a second test's thread would never observe
    // the platform bound to the first test's thread. `tests/navigation.rs`
    // hits the exact same constraint and solves it by driving every
    // scenario from one `#[test]` function on one thread; we do the same
    // here, calling the two (non-`#[test]`) scenario functions below in
    // sequence from a single `#[test]` entry point.
    #[test]
    fn pairing_wait_timer_uses_slint_mock_time() {
        i_slint_backend_testing::init_integration_test_with_mock_time();

        pairing_wait_notice_reverts_to_normal_after_the_cooldown_elapses();
        a_stale_timer_does_not_overwrite_a_newer_screen_state();
        the_result_screen_auto_returns_to_my_wallet_after_six_seconds();
        the_auto_return_timer_does_not_fire_while_on_a_different_screen();
    }

    /// Exercises the real `slint::Timer::single_shot` scheduled by
    /// `show_pairing_view`: after the cooldown, the timer must fire and put
    /// the normal instruction back.
    ///
    /// `slint::invoke_from_event_loop` (used internally by
    /// `show_pairing_view`) only queues its closure -- it runs once
    /// `slint::run_event_loop()` actually pumps the queue -- so the
    /// assertions are made from inside further queued closures rather than
    /// immediately after the call, and the result is handed back out
    /// through an `Arc<Mutex<..>>` (the closures must be `Send`, so a
    /// captured `Rc`/`RefCell` won't compile).
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
            // Immediately after `show_pairing_view`'s queued closure has
            // run, the waiting text is in effect.
            let waiting = app_weak
                .upgrade()
                .map(|a| a.get_pairing_wait_secs())
                .unwrap_or_default();

            // Advance mock time past the cooldown; the pending single-shot
            // timer fires synchronously as part of this call.
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

    /// A screen change (here, `hide_pairing_view`) that happens before the
    /// cooldown elapses must invalidate the pending timer via
    /// `PAIRING_VIEW`'s generation counter, so the timer's eventual firing
    /// does not clobber whatever later replaced the pairing-wait text.
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
            // The screen changes (e.g. pairing ends) BEFORE the cooldown
            // elapses. `hide_pairing_view` itself only queues its own
            // closure, so the follow-up step below is queued from here too
            // -- guaranteeing it runs after `hide_pairing_view`'s closure
            // (and thus after `PAIRING_VIEW.next()`) has actually executed.
            hide_pairing_view(&app_weak, "some-funding-qr-text");

            let app_weak2 = app_weak.clone();
            let final_text_clone2 = final_text_clone.clone();
            slint::invoke_from_event_loop(move || {
                // The stale timer must NOT have overwritten this later
                // status.
                if let Some(app) = app_weak2.upgrade() {
                    // A newer state, which the stale timer must not clobber.
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

    /// Exercises `app.slint`'s native `auto-return-timer` (a `Timer`
    /// declared entirely in .slint, not driven from Rust): it must not fire
    /// before its 6s interval elapses, and must switch back to
    /// `Screen::MyWallet` once it does.
    ///
    /// Unlike the pairing-timer scenarios above, this timer's binding
    /// (`running: current-screen == Screen.result`) and property
    /// read/writes here don't go through `invoke_from_event_loop` --
    /// `AppWindow::new()` runs on this same test thread, so
    /// `set_current_screen`/`get_current_screen` apply synchronously (the
    /// same direct style `tests/navigation.rs` uses inside its own
    /// event-loop-driven closures), and `mock_elapsed_time` activates any
    /// due timers synchronously too, with no running event loop required.
    fn the_result_screen_auto_returns_to_my_wallet_after_six_seconds() {
        let app = AppWindow::new().unwrap();

        app.set_current_screen(Screen::Result);

        // The `running: current-screen == Screen.result` binding is only
        // re-evaluated (and the timer actually started) the next time
        // property change handlers run, which `mock_elapsed_time` triggers
        // as a side effect. A zero-duration call here starts the timer's
        // count from "now" -- without it, the first real advance below
        // would both start the timer AND count toward its interval in one
        // step, undercounting how much mock time the timer has actually
        // seen.
        i_slint_backend_testing::mock_elapsed_time(Duration::ZERO);

        // Just under the 6s interval: the timer must not have fired yet.
        i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(5_900));
        assert_eq!(
            app.get_current_screen(),
            Screen::Result,
            "auto-return-timer fired before its 6s interval elapsed"
        );

        // Past the interval: the timer must now have fired.
        i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(200));
        assert_eq!(
            app.get_current_screen(),
            Screen::MyWallet,
            "auto-return-timer did not fire after its 6s interval elapsed"
        );
    }

    /// Guards the timer's `running: current-screen == Screen.result`
    /// binding: while on any other screen it must not run at all, so a
    /// regression that removed the binding (making the timer always fire)
    /// would be caught here.
    fn the_auto_return_timer_does_not_fire_while_on_a_different_screen() {
        let app = AppWindow::new().unwrap();

        // Deliberately NOT `Screen::MyWallet`: the timer's own effect is to
        // set the screen TO `MyWallet`, so asserting against `MyWallet`
        // itself would pass even if the timer incorrectly fired (a no-op
        // change). Using a distinct screen makes an incorrect fire visible.
        app.set_current_screen(Screen::Confirm);

        // See the sibling scenario above for why this zero-duration pump is
        // needed before the real advance: it's what makes the `running`
        // binding get (re-)evaluated, so a regression that hard-codes
        // `running: true` actually starts counting from here rather than
        // from partway into the 10s advance below.
        i_slint_backend_testing::mock_elapsed_time(Duration::ZERO);

        i_slint_backend_testing::mock_elapsed_time(Duration::from_secs(10));
        assert_eq!(
            app.get_current_screen(),
            Screen::Confirm,
            "auto-return-timer fired while not on the Result screen"
        );
    }
}
