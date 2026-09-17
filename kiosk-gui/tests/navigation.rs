slint::include_modules!();

use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use slint::platform::PointerEventButton;

#[test]
fn tapping_a_beneficiary_fires_beneficiary_selected_with_its_id() {
    i_slint_backend_testing::init_integration_test_with_system_time();

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
