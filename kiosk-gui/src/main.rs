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
    app.on_amount_selected(|index| {
        println!("amount preset selected: {index}");
    });
    app.on_family_wallet_requested(|| {
        println!("family wallet requested");
    });
    app.on_help_requested(|| {
        println!("help requested");
    });

    app.run()
}
