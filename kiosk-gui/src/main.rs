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

    app.on_beneficiary_selected(|id| {
        println!("beneficiary selected: {id}");
    });
    app.on_family_wallet_requested(|| {
        println!("family wallet requested");
    });
    app.on_help_requested(|| {
        println!("help requested");
    });

    app.run()
}
