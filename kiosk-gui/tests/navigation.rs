use kiosk_gui::slint_ui::*;

use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use slint::platform::PointerEventButton;

// This binary runs with `harness = false` (see kiosk-gui/Cargo.toml): the
// testing platform (`i_slint_backend_testing::init_integration_test_with_system_time`)
// can only be initialized once per process, but the default `#[test]`
// harness runs every test function on its own freshly spawned OS thread
// (true even under `--test-threads=1`), and the platform context is bound
// per-thread. A second test's thread would never see the first test's
// context and would fail with `NoEventLoopProvider`. Driving both
// scenarios from a single-threaded `main()` keeps everything on one
// thread, so the platform is initialized exactly once and both scenarios
// run against it in sequence.

fn tapping_a_beneficiary_fires_beneficiary_selected_with_its_id() {
    slint::spawn_local(async move {
        let app = AppWindow::new().unwrap();

        let beneficiaries = vec![
            BeneficiaryItem {
                name: "Внук Саша".into(),
                id: 1,
            },
            BeneficiaryItem {
                name: "Дочка Маша".into(),
                id: 2,
            },
        ];
        let model = std::rc::Rc::new(slint::VecModel::from(beneficiaries));
        app.set_beneficiaries(model.into());

        let selected = std::rc::Rc::new(std::cell::Cell::new(0));
        let selected_clone = selected.clone();
        app.on_beneficiary_selected(move |id| {
            selected_clone.set(id);
        });

        // `Text` elements get an implicit `accessible-role: text` /
        // `accessible-label: text` from Slint itself, so a plain label
        // match would also match the beneficiary name's child Text node
        // even before the button Rectangle is given its own
        // accessible-role/accessible-label. Filter to the button role so
        // this test genuinely exercises the beneficiary button's own
        // accessibility wiring, not the incidental label on its child text.
        let elem = ElementHandle::find_by_accessible_label(&app, "Внук Саша")
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .expect("beneficiary button for Внук Саша not found");
        elem.single_click(PointerEventButton::Left).await;

        assert_eq!(selected.get(), 1);

        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}

fn selecting_a_beneficiary_navigates_to_amount_select_with_their_name() {
    slint::spawn_local(async move {
        let app = AppWindow::new().unwrap();

        let beneficiaries = vec![BeneficiaryItem {
            name: "Внук Саша".into(),
            id: 1,
        }];
        let model = std::rc::Rc::new(slint::VecModel::from(beneficiaries));
        app.set_beneficiaries(model.into());

        {
            let app_weak = app.as_weak();
            app.on_beneficiary_selected(move |id| {
                let app = app_weak.unwrap();
                let name = if id == 1 { "Внук Саша" } else { "?" };
                app.set_selected_recipient_name(name.into());
                app.set_current_screen(Screen::AmountSelect);
            });
        }

        assert_eq!(app.get_current_screen(), Screen::MyWallet);

        let elem = ElementHandle::find_by_accessible_label(&app, "Внук Саша")
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .expect("beneficiary button not found");
        elem.single_click(PointerEventButton::Left).await;

        assert_eq!(app.get_current_screen(), Screen::AmountSelect);
        assert_eq!(
            app.get_selected_recipient_name(),
            slint::SharedString::from("Внук Саша")
        );

        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}

fn choosing_an_amount_navigates_to_confirm_with_recipient_and_amount() {
    slint::spawn_local(async move {
        let app = AppWindow::new().unwrap();
        app.set_presets(
            std::rc::Rc::new(slint::VecModel::from(vec![slint::SharedString::from(
                "10 USDC",
            )]))
            .into(),
        );
        app.set_selected_recipient_name("Внук Саша".into());

        {
            let app_weak = app.as_weak();
            app.on_amount_selected(move |index| {
                let app = app_weak.unwrap();
                let presets = ["10 USDC"];
                let amount = presets.get(index as usize).copied().unwrap_or("0");
                app.set_confirm_recipient_name(app.get_selected_recipient_name());
                app.set_confirm_amount_text(amount.into());
                app.set_current_screen(Screen::Confirm);
            });
        }

        app.set_current_screen(Screen::AmountSelect);

        let elem = ElementHandle::find_by_accessible_label(&app, "10 USDC")
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .expect("preset button not found");
        elem.single_click(PointerEventButton::Left).await;

        assert_eq!(app.get_current_screen(), Screen::Confirm);
        assert_eq!(
            app.get_confirm_recipient_name(),
            slint::SharedString::from("Внук Саша")
        );
        assert_eq!(
            app.get_confirm_amount_text(),
            slint::SharedString::from("10 USDC")
        );

        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}

fn cancelling_confirmation_returns_to_my_wallet() {
    slint::spawn_local(async move {
        let app = AppWindow::new().unwrap();
        app.set_current_screen(Screen::Confirm);

        {
            let app_weak = app.as_weak();
            app.on_send_cancelled(move || {
                let app = app_weak.unwrap();
                app.set_current_screen(Screen::MyWallet);
            });
        }

        let elem = ElementHandle::find_by_accessible_label(&app, "Cancel")
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .expect("cancel button not found");
        elem.single_click(PointerEventButton::Left).await;

        assert_eq!(app.get_current_screen(), Screen::MyWallet);

        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}

fn confirming_send_navigates_to_result_with_success_message() {
    slint::spawn_local(async move {
        let app = AppWindow::new().unwrap();
        app.set_confirm_recipient_name("Внук Саша".into());
        app.set_current_screen(Screen::Confirm);

        {
            let app_weak = app.as_weak();
            app.on_send_confirmed(move || {
                let app = app_weak.unwrap();
                let recipient = app.get_confirm_recipient_name();
                app.set_result_success(true);
                app.set_result_message(Message::Done);
                app.set_result_name(recipient);
                app.set_current_screen(Screen::Result);
            });
        }

        let elem = ElementHandle::find_by_accessible_label(&app, "Send")
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .expect("send button not found");
        elem.single_click(PointerEventButton::Left).await;

        assert_eq!(app.get_current_screen(), Screen::Result);
        assert!(app.get_result_success());
        assert_eq!(app.get_result_message(), Message::Done);
        assert_eq!(app.get_result_name(), slint::SharedString::from("Внук Саша"));
        assert!(
            ElementHandle::find_by_accessible_label(&app, "Done! Money sent to Внук Саша")
                .next()
                .is_some(),
            "the result screen must show the sentence for `Done` with the recipient's name"
        );

        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}

fn pressing_ok_on_result_returns_to_my_wallet_immediately() {
    slint::spawn_local(async move {
        let app = AppWindow::new().unwrap();
        app.set_current_screen(Screen::Result);

        {
            let app_weak = app.as_weak();
            app.on_ok_pressed(move || {
                let app = app_weak.unwrap();
                app.set_current_screen(Screen::MyWallet);
            });
        }

        let elem = ElementHandle::find_by_accessible_label(&app, "OK")
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .expect("ok button not found");
        elem.single_click(PointerEventButton::Left).await;

        assert_eq!(app.get_current_screen(), Screen::MyWallet);

        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}

// Regression test for the beneficiary-name-collision bug: when two
// beneficiaries share a display name, `on_send_confirmed` must resolve the
// wallet address using the id captured at tap time (like real main.rs does
// via `selected_beneficiary_id`), not by re-matching the confirm screen's
// name string against the model — a match-by-name would always resolve to
// the FIRST same-named entry, even if the user tapped the second one.
fn confirming_send_with_duplicate_beneficiary_names_resolves_the_tapped_id() {
    slint::spawn_local(async move {
        let app = AppWindow::new().unwrap();
        app.set_presets(
            std::rc::Rc::new(slint::VecModel::from(vec![slint::SharedString::from(
                "10 USDC",
            )]))
            .into(),
        );

        // Two beneficiaries with the SAME display name but different ids —
        // family-vault's add_beneficiary does not enforce name uniqueness.
        let beneficiaries = vec![
            BeneficiaryItem {
                name: "Иван".into(),
                id: 1,
            },
            BeneficiaryItem {
                name: "Иван".into(),
                id: 2,
            },
        ];
        let model = std::rc::Rc::new(slint::VecModel::from(beneficiaries));
        app.set_beneficiaries(model.into());

        let selected_beneficiary_id = std::rc::Rc::new(std::cell::Cell::new(0i32));

        {
            let app_weak = app.as_weak();
            let selected_beneficiary_id = selected_beneficiary_id.clone();
            app.on_beneficiary_selected(move |id| {
                let app = app_weak.unwrap();
                selected_beneficiary_id.set(id);
                app.set_selected_recipient_name("Иван".into());
                app.set_current_screen(Screen::AmountSelect);
            });
        }
        {
            let app_weak = app.as_weak();
            app.on_amount_selected(move |_index| {
                let app = app_weak.unwrap();
                app.set_confirm_recipient_name(app.get_selected_recipient_name());
                app.set_confirm_amount_text("10 USDC".into());
                app.set_current_screen(Screen::Confirm);
            });
        }

        let resolved_id = std::rc::Rc::new(std::cell::Cell::new(0i32));
        {
            let app_weak = app.as_weak();
            let selected_beneficiary_id = selected_beneficiary_id.clone();
            let resolved_id = resolved_id.clone();
            app.on_send_confirmed(move || {
                let app = app_weak.unwrap();
                // Mirrors main.rs's fix: resolve from the id captured at
                // tap time, not from a name scan.
                resolved_id.set(selected_beneficiary_id.get());
                app.set_current_screen(Screen::Result);
            });
        }

        // Tap the SECOND "Иван" (id 2). There are two buttons with the same
        // accessible label, so index into the matches explicitly.
        let elem = ElementHandle::find_by_accessible_label(&app, "Иван")
            .filter(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .nth(1)
            .expect("second Иван beneficiary button not found");
        elem.single_click(PointerEventButton::Left).await;

        assert_eq!(app.get_current_screen(), Screen::AmountSelect);
        assert_eq!(selected_beneficiary_id.get(), 2);

        let preset = ElementHandle::find_by_accessible_label(&app, "10 USDC")
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .expect("preset button not found");
        preset.single_click(PointerEventButton::Left).await;

        assert_eq!(app.get_current_screen(), Screen::Confirm);

        let send = ElementHandle::find_by_accessible_label(&app, "Send")
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .expect("send button not found");
        send.single_click(PointerEventButton::Left).await;

        assert_eq!(app.get_current_screen(), Screen::Result);
        // Must resolve to the tapped beneficiary (id 2), not the first
        // same-named entry in the model (id 1).
        assert_eq!(resolved_id.get(), 2);

        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}

fn family_wallet_button_navigates_there_and_back_button_returns() {
    slint::spawn_local(async move {
        let app = AppWindow::new().unwrap();

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

        assert_eq!(app.get_current_screen(), Screen::MyWallet);

        let to_family = ElementHandle::find_by_accessible_label(&app, "Family wallet")
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .expect("family wallet button not found");
        to_family.single_click(PointerEventButton::Left).await;
        assert_eq!(app.get_current_screen(), Screen::FamilyWallet);

        let back = ElementHandle::find_by_accessible_label(&app, "Back to my wallet")
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .expect("back button not found");
        back.single_click(PointerEventButton::Left).await;
        assert_eq!(app.get_current_screen(), Screen::MyWallet);

        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}

fn help_button_on_my_wallet_is_accessibility_discoverable() {
    slint::spawn_local(async move {
        let app = AppWindow::new().unwrap();

        let help = ElementHandle::find_by_accessible_label(&app, "Help")
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .expect("help button not found");
        assert!(help.accessible_role() == Some(AccessibleRole::Button));

        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}

fn help_button_on_family_wallet_is_accessibility_discoverable() {
    slint::spawn_local(async move {
        let app = AppWindow::new().unwrap();
        app.set_current_screen(Screen::FamilyWallet);

        let help = ElementHandle::find_by_accessible_label(&app, "Help")
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .expect("help button not found");
        assert!(help.accessible_role() == Some(AccessibleRole::Button));

        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}

fn skip_button_exists_only_while_pairing_is_active() {
    slint::spawn_local(async move {
        let app = AppWindow::new().unwrap();
        app.set_current_screen(Screen::Setup);

        assert!(
            ElementHandle::find_by_accessible_label(&app, "Skip")
                .next()
                .is_none(),
            "skip button must not exist outside pairing mode"
        );

        app.set_setup_pairing_active(true);
        assert!(
            ElementHandle::find_by_accessible_label(&app, "Skip")
                .any(|e| e.accessible_role() == Some(AccessibleRole::Button)),
            "skip button must exist as a button in pairing mode"
        );

        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}

fn tapping_skip_fires_skip_pairing_requested() {
    slint::spawn_local(async move {
        let app = AppWindow::new().unwrap();
        app.set_current_screen(Screen::Setup);
        app.set_setup_pairing_active(true);
        app.set_setup_pin_text("482 913".into());

        let fired = std::rc::Rc::new(std::cell::Cell::new(false));
        let fired_clone = fired.clone();
        app.on_skip_pairing_requested(move || fired_clone.set(true));

        let button = ElementHandle::find_by_accessible_label(&app, "Skip")
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .expect("skip button not found");
        button.single_click(PointerEventButton::Left).await;

        assert!(fired.get());

        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}

fn the_russian_translation_is_bundled_and_can_be_selected() {
    slint::spawn_local(async move {
        let app = AppWindow::new().unwrap();
        app.set_current_screen(Screen::MyWallet);

        slint::select_bundled_translation("ru")
            .expect("the Russian translation must be bundled into the binary");
        assert!(
            ElementHandle::find_by_accessible_label(&app, "Семейный кошелёк")
                .any(|e| e.accessible_role() == Some(AccessibleRole::Button)),
            "the family wallet button must be labelled in Russian after selecting ru"
        );
        assert!(
            ElementHandle::find_by_accessible_label(&app, "Family wallet")
                .next()
                .is_none(),
            "the English label must be gone after selecting ru"
        );

        // Every other scenario relies on the English source text.
        slint::select_bundled_translation("en").unwrap();
        assert!(
            ElementHandle::find_by_accessible_label(&app, "Family wallet")
                .any(|e| e.accessible_role() == Some(AccessibleRole::Button)),
            "the family wallet button must be labelled in English after selecting en"
        );

        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}

fn text_exists(app: &AppWindow, text: &str) -> bool {
    ElementHandle::find_by_accessible_label(app, text).next().is_some()
}

fn messages_are_worded_by_the_ui_in_both_languages() {
    slint::spawn_local(async move {
        let app = AppWindow::new().unwrap();

        // A failed balance fetch shows its sentence instead of the balance.
        app.set_current_screen(Screen::MyWallet);
        app.set_balance_text("120 USDC".into());
        assert!(text_exists(&app, "120 USDC"));
        app.set_balance_message(Message::NoConnection);
        assert!(text_exists(&app, "No connection. Try again later"));
        assert!(!text_exists(&app, "120 USDC"));
        app.set_balance_message(Message::None);
        assert!(text_exists(&app, "120 USDC"));

        // The result screen words `Done` with the recipient, or a fallback.
        app.set_current_screen(Screen::Result);
        app.set_result_message(Message::Done);
        app.set_result_name("Внук Саша".into());
        assert!(text_exists(&app, "Done! Money sent to Внук Саша"));
        app.set_result_name("".into());
        assert!(text_exists(&app, "Done! Money sent to Recipient"));

        // The pairing instruction follows the cooldown left.
        app.set_current_screen(Screen::Setup);
        app.set_setup_pairing_active(true);
        let normal = "Scan this code in the family app or type the digits:";
        assert!(text_exists(&app, normal));
        app.set_pairing_wait_secs(45);
        assert!(text_exists(
            &app,
            "Wait 45 sec, then scan this code in the family app or type the digits:"
        ));
        app.set_pairing_wait_secs(61);
        assert!(text_exists(
            &app,
            "Wait 2 min, then scan this code in the family app or type the digits:"
        ));
        app.set_pairing_wait_secs(0);
        assert!(text_exists(&app, normal));

        // The same states, in Russian.
        slint::select_bundled_translation("ru").unwrap();
        assert!(text_exists(
            &app,
            "Отсканируйте этот код в приложении семьи или введите цифры:"
        ));
        app.set_pairing_wait_secs(61);
        assert!(text_exists(
            &app,
            "Подождите 2 мин., затем отсканируйте этот код в приложении семьи или введите цифры:"
        ));
        app.set_current_screen(Screen::Result);
        assert!(text_exists(&app, "Готово! Деньги отправлены Получатель"));
        app.set_result_name("Внук Саша".into());
        assert!(text_exists(&app, "Готово! Деньги отправлены Внук Саша"));
        app.set_current_screen(Screen::MyWallet);
        app.set_balance_message(Message::NoConnection);
        assert!(text_exists(&app, "Нет связи. Попробуй ещё раз позже"));

        slint::select_bundled_translation("en").unwrap();
        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}

/// A button's box on the 800x480 window: `(label, x, y, width, height)`.
type ButtonBox = (String, f32, f32, f32, f32);

/// The boxes of every `KioskButton` inside the screen with element id `screen`.
fn button_boxes(app: &AppWindow, screen: &str) -> Vec<ButtonBox> {
    let root = ElementHandle::find_by_element_id(app, screen)
        .next()
        .unwrap_or_else(|| panic!("no screen {screen}"));
    root.query_descendants()
        .match_inherits("KioskButton")
        .find_all()
        .iter()
        .map(|button| {
            let position = button.absolute_position();
            let size = button.size();
            (
                button.accessible_label().unwrap_or_default().to_string(),
                position.x,
                position.y,
                size.width,
                size.height,
            )
        })
        .collect()
}

fn assert_buttons_fit(context: &str, buttons: &[ButtonBox]) {
    assert!(!buttons.is_empty(), "{context}: no buttons found");
    for (label, x, y, width, height) in buttons {
        assert!(
            *width >= 60.0 && *height >= 60.0,
            "{context}: `{label}` is {width}x{height}, under the 60px touch target"
        );
        assert!(
            *x >= 0.0 && *y >= 0.0 && x + width <= 800.0 && y + height <= 480.0,
            "{context}: `{label}` at ({x}, {y}) {width}x{height} leaves the 800x480 window"
        );
    }
    for (i, a) in buttons.iter().enumerate() {
        for b in &buttons[i + 1..] {
            let apart = a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1 || a.2 + a.4 <= b.2 || b.2 + b.4 <= a.2;
            assert!(apart, "{context}: `{}` overlaps `{}`", a.0, b.0);
        }
    }
}

fn buttons_fit_the_screen_in_both_languages_with_the_longest_names() {
    slint::spawn_local(async move {
        let app = AppWindow::new().unwrap();
        // The test backend measures text narrower than the real renderer, so
        // this checks the box rules (touch size, inside the window, no
        // overlap, centred OK); `examples/screens.rs` shows how the text
        // wraps inside them.
        let widest = "W".repeat(32);
        let names = [widest.as_str(), "Александр Константинович", "Внучка Екатерина Дм."];
        let items = |names: &[&str]| -> slint::ModelRc<BeneficiaryItem> {
            std::rc::Rc::new(slint::VecModel::from(
                names
                    .iter()
                    .zip(1..)
                    .map(|(name, id)| BeneficiaryItem { name: (*name).into(), id })
                    .collect::<Vec<_>>(),
            ))
            .into()
        };
        app.set_beneficiaries(items(&names));
        app.set_family_beneficiaries(items(&names[..2]));
        app.set_presets(
            std::rc::Rc::new(slint::VecModel::from(vec![
                "10 USDC".into(),
                "20 USDC".into(),
                "50 USDC".into(),
            ]))
            .into(),
        );
        app.set_selected_recipient_name(widest.clone().into());
        app.set_confirm_recipient_name(widest.clone().into());
        app.set_confirm_amount_text("50 USDC".into());
        app.set_result_name(widest.clone().into());
        app.set_result_message(Message::Done);
        app.set_setup_pairing_active(true);
        app.set_setup_pin_text("123 456".into());
        app.set_pairing_wait_secs(95);

        for language in ["en", "ru"] {
            slint::select_bundled_translation(language).unwrap();
            for (screen, id, buttons) in [
                (Screen::MyWallet, "AppWindow::wallet", 5),
                (Screen::FamilyWallet, "AppWindow::family-wallet", 4),
                (Screen::AmountSelect, "AppWindow::amount-select", 4),
                (Screen::Confirm, "AppWindow::confirm", 2),
                (Screen::Result, "AppWindow::result", 1),
                (Screen::Setup, "AppWindow::setup", 1),
            ] {
                app.set_current_screen(screen);
                let context = format!("{language} {id}");
                let boxes = button_boxes(&app, id);
                assert_eq!(boxes.len(), buttons, "{context}: {boxes:?}");
                assert_buttons_fit(&context, &boxes);
            }

            // The OK button sits in the middle of the window.
            app.set_current_screen(Screen::Result);
            let ok = &button_boxes(&app, "AppWindow::result")[0];
            assert!(
                (ok.1 + ok.3 / 2.0 - 400.0).abs() <= 1.0,
                "{language}: OK is not centred: {ok:?}"
            );
        }

        slint::select_bundled_translation("en").unwrap();
        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}

fn main() {
    i_slint_backend_testing::init_integration_test_with_system_time();

    tapping_a_beneficiary_fires_beneficiary_selected_with_its_id();
    println!("test tapping_a_beneficiary_fires_beneficiary_selected_with_its_id ... ok");

    selecting_a_beneficiary_navigates_to_amount_select_with_their_name();
    println!("test selecting_a_beneficiary_navigates_to_amount_select_with_their_name ... ok");

    choosing_an_amount_navigates_to_confirm_with_recipient_and_amount();
    println!("test choosing_an_amount_navigates_to_confirm_with_recipient_and_amount ... ok");

    cancelling_confirmation_returns_to_my_wallet();
    println!("test cancelling_confirmation_returns_to_my_wallet ... ok");

    confirming_send_navigates_to_result_with_success_message();
    println!("test confirming_send_navigates_to_result_with_success_message ... ok");

    pressing_ok_on_result_returns_to_my_wallet_immediately();
    println!("test pressing_ok_on_result_returns_to_my_wallet_immediately ... ok");

    confirming_send_with_duplicate_beneficiary_names_resolves_the_tapped_id();
    println!("test confirming_send_with_duplicate_beneficiary_names_resolves_the_tapped_id ... ok");

    family_wallet_button_navigates_there_and_back_button_returns();
    println!("test family_wallet_button_navigates_there_and_back_button_returns ... ok");

    help_button_on_my_wallet_is_accessibility_discoverable();
    println!("test help_button_on_my_wallet_is_accessibility_discoverable ... ok");

    help_button_on_family_wallet_is_accessibility_discoverable();
    println!("test help_button_on_family_wallet_is_accessibility_discoverable ... ok");

    skip_button_exists_only_while_pairing_is_active();
    println!("test skip_button_exists_only_while_pairing_is_active ... ok");

    tapping_skip_fires_skip_pairing_requested();
    println!("test tapping_skip_fires_skip_pairing_requested ... ok");

    the_russian_translation_is_bundled_and_can_be_selected();
    println!("test the_russian_translation_is_bundled_and_can_be_selected ... ok");

    messages_are_worded_by_the_ui_in_both_languages();
    println!("test messages_are_worded_by_the_ui_in_both_languages ... ok");

    buttons_fit_the_screen_in_both_languages_with_the_longest_names();
    println!("test buttons_fit_the_screen_in_both_languages_with_the_longest_names ... ok");

    println!();
    println!("test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s");
}
