slint::include_modules!();

mod chain;

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;

    app.set_balance_text("120 USDC".into());

    let beneficiaries = vec![
        BeneficiaryItem { name: "Внук Саша".into(), id: 1 },
        BeneficiaryItem { name: "Дочка Маша".into(), id: 2 },
    ];
    let model = std::rc::Rc::new(slint::VecModel::from(beneficiaries));
    app.set_beneficiaries(model.into());

    app.set_presets(
        std::rc::Rc::new(slint::VecModel::from(vec![
            "10 USDC".into(),
            "20 USDC".into(),
            "50 USDC".into(),
        ]))
        .into(),
    );

    app.set_family_balance_text("500 USDC".into());
    let family_beneficiaries = vec![
        BeneficiaryItem { name: "Внук Саша".into(), id: 1 },
        BeneficiaryItem { name: "Дочка Маша".into(), id: 2 },
    ];
    let family_model = std::rc::Rc::new(slint::VecModel::from(family_beneficiaries));
    app.set_family_beneficiaries(family_model.into());

    {
        let app_weak = app.as_weak();
        app.on_beneficiary_selected(move |id| {
            let app = app_weak.unwrap();
            let name = match id {
                1 => "Внук Саша",
                2 => "Дочка Маша",
                _ => "Получатель",
            };
            app.set_selected_recipient_name(name.into());
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
    {
        let app_weak = app.as_weak();
        app.on_amount_selected(move |index| {
            let app = app_weak.unwrap();
            let presets = ["10 USDC", "20 USDC", "50 USDC"];
            let amount = presets.get(index as usize).copied().unwrap_or("0 USDC");
            app.set_confirm_recipient_name(app.get_selected_recipient_name());
            app.set_confirm_amount_text(amount.into());
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
        app.on_send_confirmed(move || {
            let app = app_weak.unwrap();
            // Mock: real transaction submission is a later plan.
            let recipient = app.get_confirm_recipient_name();
            app.set_result_success(true);
            app.set_result_message_text(format!("Готово! Деньги отправлены {recipient}").into());
            app.set_current_screen(Screen::Result);
        });
    }
    {
        let app_weak = app.as_weak();
        app.on_ok_pressed(move || {
            let app = app_weak.unwrap();
            app.set_current_screen(Screen::MyWallet);
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
    app.on_family_beneficiary_selected(|id| {
        println!("family beneficiary selected: {id} — pool withdraw flow is a later plan");
    });
    app.on_help_requested(|| {
        println!("help requested");
    });
    app.on_family_help_requested(|| {
        println!("help requested (family wallet)");
    });

    app.run()
}
