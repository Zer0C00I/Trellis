//! Shows one kiosk screen filled with demo data, for presentations and for
//! checking a layout by eye.
//!
//!     cargo run -p kiosk-gui --example screens -- <screen> [ru|en] [long|wide] [name=<text>] [amount=<text>] [snapshot.ppm]
//!
//! Screens: wallet, family, amount, confirm, ok, fail, setup, wait.
//! `long` uses long names with spaces, `wide` the widest 32-character names.
//! With a file name the window is captured to a PPM file and closes.

use kiosk_gui::slint_ui::*;
use std::rc::Rc;
use std::time::Duration;

fn items(names: &[(&str, i32)]) -> slint::ModelRc<BeneficiaryItem> {
    Rc::new(slint::VecModel::from(
        names
            .iter()
            .map(|(name, id)| BeneficiaryItem {
                name: (*name).into(),
                id: *id,
            })
            .collect::<Vec<_>>(),
    ))
    .into()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let screen = args.first().map_or("wallet", String::as_str);
    let language = args
        .iter()
        .find(|a| matches!(a.as_str(), "ru" | "en"))
        .map_or("en", String::as_str);
    let long = args.iter().any(|a| a == "long");
    let wide = args.iter().any(|a| a == "wide");
    let snapshot = args.iter().find(|a| a.ends_with(".ppm"));
    let arg = |key: &str| {
        args.iter()
            .find_map(|a| a.strip_prefix(key))
            .map(str::to_owned)
    };
    let custom_name = arg("name=");
    let custom_amount = arg("amount=");

    let app = AppWindow::new()?;
    slint::select_bundled_translation(language)?;

    let widest = "W".repeat(32);
    let names: [&str; 3] = if wide {
        [&widest, &widest, &widest]
    } else if long {
        [
            "Александр Константинович Н.",
            "Внучка Екатерина Дмитриевна",
            "Сосед Пётр Иванович Сидоров",
        ]
    } else if language == "ru" {
        ["Внук Саша", "Дочка Маша", "Сосед Пётр"]
    } else {
        ["Granddaughter Anna", "Neighbour Peter", "Grandson Sasha"]
    };
    let shown = custom_name.as_deref().unwrap_or(names[0]);
    let amount = custom_amount.as_deref().unwrap_or("20 USDC");
    app.set_presets(
        Rc::new(slint::VecModel::from(vec![
            "10 USDC".into(),
            "20 USDC".into(),
            "50 USDC".into(),
        ]))
        .into(),
    );
    app.set_balance_text("120 USDC".into());
    app.set_family_balance_text("340 USDC".into());
    app.set_beneficiaries(items(&[(names[0], 1), (names[1], 2), (names[2], 3)]));
    app.set_family_beneficiaries(items(&[(names[0], 1), (names[1], 2)]));
    app.set_selected_recipient_name(shown.into());
    app.set_confirm_recipient_name(shown.into());
    app.set_confirm_amount_text(amount.into());
    app.set_result_name(shown.into());
    match screen {
        "wallet" => app.set_current_screen(Screen::MyWallet),
        "family" => app.set_current_screen(Screen::FamilyWallet),
        "amount" => app.set_current_screen(Screen::AmountSelect),
        "confirm" => app.set_current_screen(Screen::Confirm),
        "ok" => {
            app.set_result_success(true);
            app.set_result_message(Message::Done);
            app.set_current_screen(Screen::Result);
        }
        "fail" => {
            app.set_result_success(false);
            app.set_result_message(Message::NoConnection);
            app.set_current_screen(Screen::Result);
        }
        "setup" | "wait" => {
            app.set_setup_qr_image(demo_qr("fvpair1:00112233445566778899aabbccddeeff"));
            app.set_setup_pairing_active(true);
            app.set_setup_pin_text("123 456".into());
            app.set_pairing_wait_secs(if screen == "wait" { 95 } else { 0 });
            app.set_current_screen(Screen::Setup);
        }
        other => return Err(format!("unknown screen {other}").into()),
    }

    if let Some(path) = snapshot {
        let path = path.clone();
        let weak = app.as_weak();
        slint::Timer::single_shot(Duration::from_millis(600), move || {
            if let Some(app) = weak.upgrade() {
                let shot = app.window().take_snapshot().expect("window snapshot");
                let mut out = format!("P6\n{} {}\n255\n", shot.width(), shot.height()).into_bytes();
                out.extend(shot.as_bytes().chunks(4).flat_map(|px| px[..3].to_vec()));
                std::fs::write(&path, out).expect("write snapshot");
            }
            slint::quit_event_loop().ok();
        });
    }
    app.run()?;
    Ok(())
}

fn demo_qr(data: &str) -> slint::Image {
    let code = qrcode::QrCode::new(data.as_bytes()).expect("QR encoding failed");
    let (width, scale, border) = (code.width(), 8usize, 4usize);
    let size = (width + border * 2) * scale;
    let mut buffer = slint::SharedPixelBuffer::<slint::Rgb8Pixel>::new(size as u32, size as u32);
    let stride = size;
    let bytes = buffer.make_mut_bytes();
    for y in 0..size {
        for x in 0..size {
            let (mx, my) = (x / scale, y / scale);
            let dark = mx >= border
                && my >= border
                && mx - border < width
                && my - border < width
                && code[(mx - border, my - border)] == qrcode::Color::Dark;
            let value: u8 = if dark { 0 } else { 255 };
            let i = (y * stride + x) * 3;
            bytes[i..i + 3].fill(value);
        }
    }
    slint::Image::from_rgb8(buffer)
}
