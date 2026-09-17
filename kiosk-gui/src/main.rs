slint::include_modules!();

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
    app.on_send_confirmed(|| {
        println!("send confirmed — wired to real transactions in a later plan");
    });
    app.on_family_wallet_requested(|| {
        println!("family wallet requested");
    });
    app.on_help_requested(|| {
        println!("help requested");
    });

    app.run()
}
