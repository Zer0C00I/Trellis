slint::include_modules!();

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
            BeneficiaryItem { name: "Внук Саша".into(), id: 1 },
            BeneficiaryItem { name: "Дочка Маша".into(), id: 2 },
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

        let beneficiaries = vec![BeneficiaryItem { name: "Внук Саша".into(), id: 1 }];
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
        assert_eq!(app.get_confirm_recipient_name(), slint::SharedString::from("Внук Саша"));
        assert_eq!(app.get_confirm_amount_text(), slint::SharedString::from("10 USDC"));

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

        let elem = ElementHandle::find_by_accessible_label(&app, "Отмена")
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
                app.set_result_message_text(
                    format!("Готово! Деньги отправлены {recipient}").into(),
                );
                app.set_current_screen(Screen::Result);
            });
        }

        let elem = ElementHandle::find_by_accessible_label(&app, "Отправить")
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .expect("send button not found");
        elem.single_click(PointerEventButton::Left).await;

        assert_eq!(app.get_current_screen(), Screen::Result);
        assert!(app.get_result_success());
        assert_eq!(
            app.get_result_message_text(),
            slint::SharedString::from("Готово! Деньги отправлены Внук Саша")
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

        let elem = ElementHandle::find_by_accessible_label(&app, "Ок")
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .expect("ok button not found");
        elem.single_click(PointerEventButton::Left).await;

        assert_eq!(app.get_current_screen(), Screen::MyWallet);

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

        let to_family = ElementHandle::find_by_accessible_label(&app, "Семейный кошелёк")
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .expect("family wallet button not found");
        to_family.single_click(PointerEventButton::Left).await;
        assert_eq!(app.get_current_screen(), Screen::FamilyWallet);

        let back = ElementHandle::find_by_accessible_label(&app, "Назад к моему кошельку")
            .find(|e| e.accessible_role() == Some(AccessibleRole::Button))
            .expect("back button not found");
        back.single_click(PointerEventButton::Left).await;
        assert_eq!(app.get_current_screen(), Screen::MyWallet);

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

    family_wallet_button_navigates_there_and_back_button_returns();
    println!("test family_wallet_button_navigates_there_and_back_button_returns ... ok");

    println!();
    println!("test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s");
}
