# Kiosk GUI Shell Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Slint UI shell — all 5 screens from the design spec,
navigation between them, and the WCAG AAA visual theme (60px targets,
16px spacing, 7:1-contrast black/white palette) — driven by mock data.
No blockchain code in this plan; that's a follow-up plan once the shell
is demoable and reviewed.

**Architecture:** One new Cargo workspace member, `kiosk-gui`, alongside
`programs/family-vault`. `.slint` files define each screen as a
component; a root `AppWindow` component switches between them via an
`enum`-typed state property. Rust (`main.rs`) owns navigation callbacks
and (for this plan) hardcoded mock data — real Solana reads/writes are
out of scope here, per the design spec's own "Open Questions" split.

**Tech Stack:** Rust, Slint 1.18.0 (`i-slint-backend-winit`, default
desktop backend for dev — `backend-winit-wayland` for real hardware is
a later plan's concern), `i-slint-backend-testing` 1.16.0 for headless
UI tests (confirmed real, published crate — see spec Sources for the
adjacent research trail; this plan's own verification: `docs.rs`
listing for `i-slint-backend-testing` shows `init_integration_test_with_system_time()`,
`ElementHandle::find_by_element_id`, `single_click`, all used below
exactly as documented).

**Spec:** `docs/superpowers/specs/2026-09-17-kiosk-gui-design.md`

## Global Constraints

- Touch target size: **60×60px minimum**, **16px spacing** between
  adjacent targets (spec: exceeds WCAG 2.2 AAA's 44×44px floor,
  empirically reduces mis-taps ~70% vs. 44px).
- Contrast: use pure black (`#000000`) on pure white (`#FFFFFF`) for
  all interactive elements and body text — 21:1 ratio, clears the
  AAA 7:1 requirement with margin, and sidesteps per-color contrast
  calculation entirely. Do not introduce blue as a functional color
  (spec: fades for elderly users).
- No jargon on any screen: forbidden words are "tx hash", "PDA",
  "confirmed on-chain", "wallet address", "seed", "gas", "network fee".
- No screen auto-advances except the Result screen (spec: 2.2.4/3.2.5 —
  changes only by user request, with the one documented exception).
- No text-as-image — all text is real Slint `Text` elements.
- Every screen has a "Помощь" (Help) button in the same fixed position
  (bottom-right) — spec: 3.2.6 Consistent Help. This plan wires it as a
  callback that's a no-op for now (real call-relative logic is a later
  plan); the button's presence and fixed position are what this plan
  must deliver.
- Do not modify anything under `programs/family-vault/` — this plan is
  additive, a new workspace member only.

---

## File Structure

```
solana/
├── Cargo.toml                      # MODIFY: add "kiosk-gui" to members
└── kiosk-gui/
    ├── Cargo.toml                  # CREATE
    ├── build.rs                    # CREATE
    ├── src/
    │   └── main.rs                 # CREATE: navigation state, mock data, event loop
    ├── ui/
    │   ├── theme.slint             # CREATE: colors, sizes (global singleton)
    │   ├── app.slint               # CREATE: root window, screen switcher
    │   ├── my-wallet.slint         # CREATE
    │   ├── amount-select.slint     # CREATE
    │   ├── confirm.slint           # CREATE
    │   ├── result.slint            # CREATE
    │   └── family-wallet.slint     # CREATE
    └── tests/
        └── navigation.rs           # CREATE: i-slint-backend-testing based tests
```

---

### Task 1: Workspace scaffold + theme

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `kiosk-gui/Cargo.toml`
- Create: `kiosk-gui/build.rs`
- Create: `kiosk-gui/ui/theme.slint`
- Create: `kiosk-gui/ui/app.slint` (minimal placeholder — real screens added in later tasks)
- Create: `kiosk-gui/src/main.rs` (minimal — shows the placeholder window)

**Interfaces:**
- Produces: `global Theme` singleton in `theme.slint` with properties
  `bg: color`, `fg: color`, `button-bg: color`, `button-fg: color`,
  `secondary-button-bg: color`, `secondary-button-border: color`,
  `success-color: color`, `error-color: color`, `target-size: length`,
  `spacing: length`, `font-size-heading: length`, `font-size-body: length`.
  Every later task imports this global.

- [ ] **Step 1: Add `kiosk-gui` to the workspace root `Cargo.toml`**

Modify `/home/user/Documents/Projects/rust/solana/Cargo.toml`:

```toml
[workspace]
members = ["programs/family-vault", "kiosk-gui"]
resolver = "2"
```

- [ ] **Step 2: Create `kiosk-gui/Cargo.toml`**

```toml
[package]
name = "kiosk-gui"
version = "0.1.0"
edition = "2021"

[dependencies]
slint = "1.18.0"

[build-dependencies]
slint-build = "1.18.0"

[dev-dependencies]
i-slint-backend-testing = "1.16.0"
```

- [ ] **Step 3: Create `kiosk-gui/build.rs`**

```rust
fn main() {
    // ElementHandle-based tests (Task 2+) require debug info in the
    // generated code — without this, i-slint-backend-testing fails at
    // runtime with "requires the presence of debug info". Emitting it
    // unconditionally is harmless for the release build.
    let config = slint_build::CompilerConfiguration::new().with_debug_info(true);
    slint_build::compile_with_config("ui/app.slint", config).unwrap();
}
```

- [ ] **Step 4: Create `kiosk-gui/ui/theme.slint`**

```slint
export global Theme {
    // Pure black-on-white: 21:1 contrast, clears WCAG 2.2 AAA's 7:1
    // floor with margin, no per-color contrast math needed.
    out property <color> bg: #ffffff;
    out property <color> fg: #000000;

    out property <color> button-bg: #000000;
    out property <color> button-fg: #ffffff;

    out property <color> secondary-button-bg: #ffffff;
    out property <color> secondary-button-border: #000000;
    out property <color> secondary-button-fg: #000000;

    out property <color> success-color: #1b5e20;
    out property <color> error-color: #b71c1c;

    // Spec: 60x60px minimum target size, 16px spacing — exceeds the
    // WCAG 2.2 AAA 44x44px floor (empirically ~70% fewer mis-taps).
    out property <length> target-size: 60px;
    out property <length> spacing: 16px;

    out property <length> font-size-heading: 40px;
    out property <length> font-size-body: 28px;
}
```

- [ ] **Step 5: Create a minimal `kiosk-gui/ui/app.slint`**

```slint
import { Theme } from "theme.slint";

export component AppWindow inherits Window {
    width: 800px;
    height: 480px;
    background: Theme.bg;

    Text {
        text: "Family Vault";
        color: Theme.fg;
        font-size: Theme.font-size-heading;
    }
}
```

- [ ] **Step 6: Create `kiosk-gui/src/main.rs`**

```rust
slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;
    app.run()
}
```

- [ ] **Step 7: Verify it builds**

Run: `cargo build --manifest-path kiosk-gui/Cargo.toml`
Expected: build succeeds

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml kiosk-gui
git commit -m "chore: scaffold kiosk-gui Slint workspace member with WCAG AAA theme"
```

---

### Task 2: My Wallet screen (home)

**Files:**
- Create: `kiosk-gui/ui/my-wallet.slint`
- Modify: `kiosk-gui/ui/app.slint`
- Modify: `kiosk-gui/src/main.rs`
- Create: `kiosk-gui/tests/navigation.rs`

**Interfaces:**
- Consumes: `Theme` global (Task 1)
- Produces: `export struct BeneficiaryItem { name: string, id: int }`
  (in `my-wallet.slint`, re-exported through `app.slint` so Rust's
  `slint::include_modules!()` generates a top-level `BeneficiaryItem`
  type — later tasks that add beneficiary data reuse this exact
  struct); `export component MyWalletScreen` with properties
  `in property <string> balance-text`, `in property <[BeneficiaryItem]> beneficiaries`,
  and callbacks `callback beneficiary-selected(int)`,
  `callback family-wallet-requested()`, `callback help-requested()`.

- [ ] **Step 1: Write `my-wallet.slint`**

Create `kiosk-gui/ui/my-wallet.slint`:

```slint
import { Theme } from "theme.slint";

export struct BeneficiaryItem {
    name: string,
    id: int,
}

export component MyWalletScreen inherits Rectangle {
    in property <string> balance-text: "0";
    in property <[BeneficiaryItem]> beneficiaries: [];

    callback beneficiary-selected(int);
    callback family-wallet-requested();
    callback help-requested();

    background: Theme.bg;

    VerticalLayout {
        padding: Theme.spacing;
        spacing: Theme.spacing;

        Text {
            text: "Мой кошелёк";
            color: Theme.fg;
            font-size: Theme.font-size-heading;
        }

        Text {
            text: balance-text;
            color: Theme.fg;
            font-size: Theme.font-size-heading;
        }

        for b in beneficiaries: Rectangle {
            height: Theme.target-size;
            background: Theme.button-bg;
            border-radius: 8px;

            TouchArea {
                clicked => { beneficiary-selected(b.id); }
            }

            Text {
                text: b.name;
                color: Theme.button-fg;
                font-size: Theme.font-size-body;
                horizontal-alignment: center;
                vertical-alignment: center;
            }
        }
    }

    Rectangle {
        // Family wallet button — top area, visually distinct from the
        // beneficiary list (spec: not the same visual weight as sending
        // money, since it navigates rather than moves funds).
        x: parent.width - self.width - Theme.spacing;
        y: Theme.spacing;
        width: 180px;
        height: Theme.target-size;
        background: Theme.secondary-button-bg;
        border-color: Theme.secondary-button-border;
        border-width: 2px;
        border-radius: 8px;

        TouchArea {
            clicked => { family-wallet-requested(); }
        }

        Text {
            text: "Семейный кошелёк";
            color: Theme.secondary-button-fg;
            font-size: 20px;
            horizontal-alignment: center;
            vertical-alignment: center;
            wrap: word-wrap;
        }
    }

    Rectangle {
        // Help button — fixed bottom-right on every screen (spec: 3.2.6).
        x: parent.width - self.width - Theme.spacing;
        y: parent.height - self.height - Theme.spacing;
        width: Theme.target-size;
        height: Theme.target-size;
        background: Theme.secondary-button-bg;
        border-color: Theme.secondary-button-border;
        border-width: 2px;
        border-radius: 8px;

        TouchArea {
            clicked => { help-requested(); }
        }

        Text {
            text: "?";
            color: Theme.secondary-button-fg;
            font-size: Theme.font-size-heading;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }
}
```

- [ ] **Step 2: Wire it into `app.slint`**

Replace the contents of `kiosk-gui/ui/app.slint`:

```slint
import { Theme } from "theme.slint";
import { MyWalletScreen, BeneficiaryItem } from "my-wallet.slint";

export { BeneficiaryItem }

export component AppWindow inherits Window {
    width: 800px;
    height: 480px;
    background: Theme.bg;

    in property <string> balance-text <=> wallet.balance-text;
    in property <[BeneficiaryItem]> beneficiaries <=> wallet.beneficiaries;

    callback beneficiary-selected <=> wallet.beneficiary-selected;
    callback family-wallet-requested <=> wallet.family-wallet-requested;
    callback help-requested <=> wallet.help-requested;

    wallet := MyWalletScreen {
        width: 100%;
        height: 100%;
    }
}
```

- [ ] **Step 3: Wire mock data into `main.rs`**

Replace `kiosk-gui/src/main.rs`:

```rust
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
```

- [ ] **Step 4: Verify it builds and runs**

Run: `cargo build --manifest-path kiosk-gui/Cargo.toml`
Expected: build succeeds

- [ ] **Step 5: Write the failing test**

Create `kiosk-gui/tests/navigation.rs`:

```rust
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
```

- [ ] **Step 6: Run test to verify it fails**

Run: `cargo test --manifest-path kiosk-gui/Cargo.toml --test navigation -- --nocapture`
Expected: FAIL — `find_by_accessible_label` finds nothing, because
`Text` elements aren't accessible-labeled by default; this confirms the
test actually exercises real UI lookup, not a tautology. Fix in the
next step.

- [ ] **Step 7: Add accessibility labels so the test (and real screen
readers / the AAA target) can find the buttons**

In `kiosk-gui/ui/my-wallet.slint`, add `accessible-role` and
`accessible-label` to the beneficiary button `Rectangle`:

```slint
        for b in beneficiaries: Rectangle {
            height: Theme.target-size;
            background: Theme.button-bg;
            border-radius: 8px;
            accessible-role: button;
            accessible-label: b.name;

            TouchArea {
                clicked => { beneficiary-selected(b.id); }
            }
```

(Replace the existing beneficiary-button block with this version — the
only change is the two added `accessible-*` lines.)

- [ ] **Step 8: Run test to verify it passes**

Run: `cargo test --manifest-path kiosk-gui/Cargo.toml --test navigation -- --nocapture`
Expected: PASS

- [ ] **Step 9: Commit**

```bash
git add kiosk-gui
git commit -m "feat: add My Wallet screen with beneficiary list and navigation test"
```

---

### Task 3: Amount Select screen

**Files:**
- Create: `kiosk-gui/ui/amount-select.slint`
- Modify: `kiosk-gui/ui/app.slint`
- Modify: `kiosk-gui/src/main.rs`
- Modify: `kiosk-gui/tests/navigation.rs`

**Interfaces:**
- Consumes: `Theme` (Task 1)
- Produces: `export component AmountSelectScreen` with properties
  `in property <string> recipient-name`, `in property <[string]> presets`,
  and callbacks `callback amount-selected(int)`, `callback back-requested()`.
  (`amount-selected` carries the INDEX into `presets`, not a currency
  value — Rust owns the mapping from index to actual amount, since the
  presets are configured by a relative, not hardcoded here.)

- [ ] **Step 1: Write `amount-select.slint`**

Create `kiosk-gui/ui/amount-select.slint`:

```slint
import { Theme } from "theme.slint";

export component AmountSelectScreen inherits Rectangle {
    in property <string> recipient-name: "";
    in property <[string]> presets: [];

    callback amount-selected(int);
    callback back-requested();

    background: Theme.bg;

    VerticalLayout {
        padding: Theme.spacing;
        spacing: Theme.spacing;

        Text {
            text: "Сколько отправить " + recipient-name + "?";
            color: Theme.fg;
            font-size: Theme.font-size-heading;
            wrap: word-wrap;
        }

        for amount[i] in presets: Rectangle {
            height: Theme.target-size;
            background: Theme.button-bg;
            border-radius: 8px;
            accessible-role: button;
            accessible-label: amount;

            TouchArea {
                clicked => { amount-selected(i); }
            }

            Text {
                text: amount;
                color: Theme.button-fg;
                font-size: Theme.font-size-body;
                horizontal-alignment: center;
                vertical-alignment: center;
            }
        }
    }

    Rectangle {
        x: Theme.spacing;
        y: parent.height - self.height - Theme.spacing;
        width: 140px;
        height: Theme.target-size;
        background: Theme.secondary-button-bg;
        border-color: Theme.secondary-button-border;
        border-width: 2px;
        border-radius: 8px;
        accessible-role: button;
        accessible-label: "Назад";

        TouchArea {
            clicked => { back-requested(); }
        }

        Text {
            text: "Назад";
            color: Theme.secondary-button-fg;
            font-size: Theme.font-size-body;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }
}
```

- [ ] **Step 2: Wire a screen switcher into `app.slint`**

Replace `kiosk-gui/ui/app.slint` entirely — this introduces the
`current-screen` state that later tasks extend:

```slint
import { Theme } from "theme.slint";
import { MyWalletScreen, BeneficiaryItem } from "my-wallet.slint";
import { AmountSelectScreen } from "amount-select.slint";

export { BeneficiaryItem }

export enum Screen { my-wallet, amount-select }

export component AppWindow inherits Window {
    width: 800px;
    height: 480px;
    background: Theme.bg;

    in-out property <Screen> current-screen: Screen.my-wallet;

    in property <string> balance-text <=> wallet.balance-text;
    in property <[BeneficiaryItem]> beneficiaries <=> wallet.beneficiaries;
    in property <string> selected-recipient-name <=> amount-select.recipient-name;
    in property <[string]> presets <=> amount-select.presets;

    callback beneficiary-selected <=> wallet.beneficiary-selected;
    callback family-wallet-requested <=> wallet.family-wallet-requested;
    callback help-requested <=> wallet.help-requested;
    callback amount-selected <=> amount-select.amount-selected;
    callback back-requested <=> amount-select.back-requested;

    wallet := MyWalletScreen {
        width: 100%;
        height: 100%;
        visible: current-screen == Screen.my-wallet;
    }

    amount-select := AmountSelectScreen {
        width: 100%;
        height: 100%;
        visible: current-screen == Screen.amount-select;
    }
}
```

- [ ] **Step 3: Drive the state transition from Rust**

Replace `kiosk-gui/src/main.rs`:

```rust
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
```

- [ ] **Step 4: Verify it builds**

Run: `cargo build --manifest-path kiosk-gui/Cargo.toml`
Expected: build succeeds

- [ ] **Step 5: Write the failing test**

Append to `kiosk-gui/tests/navigation.rs`:

```rust
#[test]
fn selecting_a_beneficiary_navigates_to_amount_select_with_their_name() {
    i_slint_backend_testing::init_integration_test_with_system_time();

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
        assert_eq!(app.get_selected_recipient_name(), "Внук Саша".into());

        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test --manifest-path kiosk-gui/Cargo.toml --test navigation -- --nocapture`
Expected: both tests PASS

- [ ] **Step 7: Commit**

```bash
git add kiosk-gui
git commit -m "feat: add Amount Select screen with screen-switching state machine"
```

---

### Task 4: Confirmation screen

**Files:**
- Create: `kiosk-gui/ui/confirm.slint`
- Modify: `kiosk-gui/ui/app.slint`
- Modify: `kiosk-gui/src/main.rs`
- Modify: `kiosk-gui/tests/navigation.rs`

**Interfaces:**
- Consumes: `Theme` (Task 1), `Screen` enum (Task 3)
- Produces: `export component ConfirmScreen` with properties
  `in property <string> recipient-name`, `in property <string> amount-text`,
  and callbacks `callback send-confirmed()`, `callback send-cancelled()`.

This screen is mandated by the design spec's WCAG 3.3.4/3.3.6 —
required before any irreversible action, never skipped.

- [ ] **Step 1: Write `confirm.slint`**

Create `kiosk-gui/ui/confirm.slint`:

```slint
import { Theme } from "theme.slint";

export component ConfirmScreen inherits Rectangle {
    in property <string> recipient-name: "";
    in property <string> amount-text: "";

    callback send-confirmed();
    callback send-cancelled();

    background: Theme.bg;

    VerticalLayout {
        padding: Theme.spacing;
        spacing: Theme.spacing;

        Text {
            text: "Отправить " + amount-text + " → " + recipient-name + "?";
            color: Theme.fg;
            font-size: Theme.font-size-heading;
            wrap: word-wrap;
        }

        HorizontalLayout {
            spacing: Theme.spacing;

            Rectangle {
                height: Theme.target-size;
                background: Theme.button-bg;
                border-radius: 8px;
                accessible-role: button;
                accessible-label: "Отправить";

                TouchArea {
                    clicked => { send-confirmed(); }
                }

                Text {
                    text: "Отправить";
                    color: Theme.button-fg;
                    font-size: Theme.font-size-body;
                    horizontal-alignment: center;
                    vertical-alignment: center;
                }
            }

            Rectangle {
                height: Theme.target-size;
                background: Theme.secondary-button-bg;
                border-color: Theme.secondary-button-border;
                border-width: 2px;
                border-radius: 8px;
                accessible-role: button;
                accessible-label: "Отмена";

                TouchArea {
                    clicked => { send-cancelled(); }
                }

                Text {
                    text: "Отмена";
                    color: Theme.secondary-button-fg;
                    font-size: Theme.font-size-body;
                    horizontal-alignment: center;
                    vertical-alignment: center;
                }
            }
        }
    }
}
```

Note: the two buttons are the same size and neither is visually
dominant (spec: don't bias toward accidental confirmation) — both live
in a `HorizontalLayout` that gives them equal width by default.

- [ ] **Step 2: Wire it into `app.slint`**

Replace `kiosk-gui/ui/app.slint`:

```slint
import { Theme } from "theme.slint";
import { MyWalletScreen, BeneficiaryItem } from "my-wallet.slint";
import { AmountSelectScreen } from "amount-select.slint";
import { ConfirmScreen } from "confirm.slint";

export { BeneficiaryItem }

export enum Screen { my-wallet, amount-select, confirm }

export component AppWindow inherits Window {
    width: 800px;
    height: 480px;
    background: Theme.bg;

    in-out property <Screen> current-screen: Screen.my-wallet;

    in property <string> balance-text <=> wallet.balance-text;
    in property <[BeneficiaryItem]> beneficiaries <=> wallet.beneficiaries;
    in property <string> selected-recipient-name <=> amount-select.recipient-name;
    in property <[string]> presets <=> amount-select.presets;
    in property <string> confirm-recipient-name <=> confirm.recipient-name;
    in property <string> confirm-amount-text <=> confirm.amount-text;

    callback beneficiary-selected <=> wallet.beneficiary-selected;
    callback family-wallet-requested <=> wallet.family-wallet-requested;
    callback help-requested <=> wallet.help-requested;
    callback amount-selected <=> amount-select.amount-selected;
    callback back-requested <=> amount-select.back-requested;
    callback send-confirmed <=> confirm.send-confirmed;
    callback send-cancelled <=> confirm.send-cancelled;

    wallet := MyWalletScreen {
        width: 100%;
        height: 100%;
        visible: current-screen == Screen.my-wallet;
    }

    amount-select := AmountSelectScreen {
        width: 100%;
        height: 100%;
        visible: current-screen == Screen.amount-select;
    }

    confirm := ConfirmScreen {
        width: 100%;
        height: 100%;
        visible: current-screen == Screen.confirm;
    }
}
```

- [ ] **Step 3: Drive the transition from Rust**

In `kiosk-gui/src/main.rs`, replace the `app.on_amount_selected(...)`
block (currently just a `println!`) with:

```rust
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
```

- [ ] **Step 4: Verify it builds**

Run: `cargo build --manifest-path kiosk-gui/Cargo.toml`
Expected: build succeeds

- [ ] **Step 5: Write the failing test**

Append to `kiosk-gui/tests/navigation.rs`:

```rust
#[test]
fn choosing_an_amount_navigates_to_confirm_with_recipient_and_amount() {
    i_slint_backend_testing::init_integration_test_with_system_time();

    slint::spawn_local(async move {
        let app = AppWindow::new().unwrap();
        app.set_presets(
            std::rc::Rc::new(slint::VecModel::from(vec!["10 USDC".to_string()])).into(),
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
        assert_eq!(app.get_confirm_recipient_name(), "Внук Саша".into());
        assert_eq!(app.get_confirm_amount_text(), "10 USDC".into());

        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}

#[test]
fn cancelling_confirmation_returns_to_my_wallet() {
    i_slint_backend_testing::init_integration_test_with_system_time();

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
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --manifest-path kiosk-gui/Cargo.toml --test navigation -- --nocapture`
Expected: all 4 tests PASS

- [ ] **Step 7: Commit**

```bash
git add kiosk-gui
git commit -m "feat: add Confirm screen (WCAG 3.3.4/3.3.6 mandatory confirmation)"
```

---

### Task 5: Result screen

**Files:**
- Create: `kiosk-gui/ui/result.slint`
- Modify: `kiosk-gui/ui/app.slint`
- Modify: `kiosk-gui/src/main.rs`
- Modify: `kiosk-gui/tests/navigation.rs`

**Interfaces:**
- Consumes: `Theme` (Task 1), `Screen` enum (Task 3)
- Produces: `export component ResultScreen` with properties
  `in property <bool> success`, `in property <string> message-text`,
  and callback `callback ok-pressed()`.

Per spec: auto-return to My Wallet after 5-8 seconds, but the "Ок"
button works immediately without waiting — this plan implements the
button; the auto-return timer is Step 3 below using Slint's built-in
`Timer` element (confirmed real, standard Slint language feature, not
guessed).

- [ ] **Step 1: Write `result.slint`**

Create `kiosk-gui/ui/result.slint`:

```slint
import { Theme } from "theme.slint";

export component ResultScreen inherits Rectangle {
    in property <bool> success: true;
    in property <string> message-text: "";

    callback ok-pressed();

    background: Theme.bg;

    VerticalLayout {
        padding: Theme.spacing;
        spacing: Theme.spacing;
        alignment: center;

        Text {
            text: success ? "✓" : "✗";
            color: success ? Theme.success-color : Theme.error-color;
            font-size: 96px;
            horizontal-alignment: center;
        }

        Text {
            text: message-text;
            color: Theme.fg;
            font-size: Theme.font-size-heading;
            horizontal-alignment: center;
            wrap: word-wrap;
        }

        Rectangle {
            height: Theme.target-size;
            width: 200px;
            background: Theme.button-bg;
            border-radius: 8px;
            accessible-role: button;
            accessible-label: "Ок";

            TouchArea {
                clicked => { ok-pressed(); }
            }

            Text {
                text: "Ок";
                color: Theme.button-fg;
                font-size: Theme.font-size-body;
                horizontal-alignment: center;
                vertical-alignment: center;
            }
        }
    }
}
```

- [ ] **Step 2: Wire it into `app.slint`**

Replace `kiosk-gui/ui/app.slint`:

```slint
import { Theme } from "theme.slint";
import { MyWalletScreen, BeneficiaryItem } from "my-wallet.slint";
import { AmountSelectScreen } from "amount-select.slint";
import { ConfirmScreen } from "confirm.slint";
import { ResultScreen } from "result.slint";

export { BeneficiaryItem }

export enum Screen { my-wallet, amount-select, confirm, result }

export component AppWindow inherits Window {
    width: 800px;
    height: 480px;
    background: Theme.bg;

    in-out property <Screen> current-screen: Screen.my-wallet;

    in property <string> balance-text <=> wallet.balance-text;
    in property <[BeneficiaryItem]> beneficiaries <=> wallet.beneficiaries;
    in property <string> selected-recipient-name <=> amount-select.recipient-name;
    in property <[string]> presets <=> amount-select.presets;
    in property <string> confirm-recipient-name <=> confirm.recipient-name;
    in property <string> confirm-amount-text <=> confirm.amount-text;
    in property <bool> result-success <=> result.success;
    in property <string> result-message-text <=> result.message-text;

    callback beneficiary-selected <=> wallet.beneficiary-selected;
    callback family-wallet-requested <=> wallet.family-wallet-requested;
    callback help-requested <=> wallet.help-requested;
    callback amount-selected <=> amount-select.amount-selected;
    callback back-requested <=> amount-select.back-requested;
    callback send-confirmed <=> confirm.send-confirmed;
    callback send-cancelled <=> confirm.send-cancelled;
    callback ok-pressed <=> result.ok-pressed;

    wallet := MyWalletScreen {
        width: 100%;
        height: 100%;
        visible: current-screen == Screen.my-wallet;
    }

    amount-select := AmountSelectScreen {
        width: 100%;
        height: 100%;
        visible: current-screen == Screen.amount-select;
    }

    confirm := ConfirmScreen {
        width: 100%;
        height: 100%;
        visible: current-screen == Screen.confirm;
    }

    result := ResultScreen {
        width: 100%;
        height: 100%;
        visible: current-screen == Screen.result;
    }

    auto-return-timer := Timer {
        interval: 6s;
        running: current-screen == Screen.result;
        triggered => {
            current-screen = Screen.my-wallet;
        }
    }
}
```

- [ ] **Step 3: Drive it from Rust**

In `kiosk-gui/src/main.rs`, replace the `app.on_send_confirmed(...)`
block with:

```rust
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
```

- [ ] **Step 4: Verify it builds**

Run: `cargo build --manifest-path kiosk-gui/Cargo.toml`
Expected: build succeeds

- [ ] **Step 5: Write the failing tests**

Append to `kiosk-gui/tests/navigation.rs`:

```rust
#[test]
fn confirming_send_navigates_to_result_with_success_message() {
    i_slint_backend_testing::init_integration_test_with_system_time();

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
            "Готово! Деньги отправлены Внук Саша".into()
        );

        slint::quit_event_loop().unwrap();
    })
    .unwrap();
    slint::run_event_loop().unwrap();
}

#[test]
fn pressing_ok_on_result_returns_to_my_wallet_immediately() {
    i_slint_backend_testing::init_integration_test_with_system_time();

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
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --manifest-path kiosk-gui/Cargo.toml --test navigation -- --nocapture`
Expected: all 6 tests PASS

- [ ] **Step 7: Commit**

```bash
git add kiosk-gui
git commit -m "feat: add Result screen with 6s auto-return timer and immediate Ok button"
```

---

### Task 6: Family Wallet screen

**Files:**
- Create: `kiosk-gui/ui/family-wallet.slint`
- Modify: `kiosk-gui/ui/app.slint`
- Modify: `kiosk-gui/src/main.rs`
- Modify: `kiosk-gui/tests/navigation.rs`

**Interfaces:**
- Consumes: `Theme` (Task 1), `Screen` enum (Task 5), `BeneficiaryItem` (Task 2)
- Produces: `export component FamilyWalletScreen` with properties
  `in property <string> balance-text`, `in property <[BeneficiaryItem]> beneficiaries`,
  and callbacks `callback beneficiary-selected(int)`, `callback back-to-my-wallet-requested()`,
  `callback help-requested()`.

Mirrors `MyWalletScreen`'s layout exactly (spec: same pattern, for
predictability — WCAG 3.2.3/3.2.4), swapping only the heading text and
the navigation button (which goes back to My Wallet instead of forward
to Family Wallet).

- [ ] **Step 1: Write `family-wallet.slint`**

Create `kiosk-gui/ui/family-wallet.slint`:

```slint
import { Theme } from "theme.slint";
import { BeneficiaryItem } from "my-wallet.slint";

export component FamilyWalletScreen inherits Rectangle {
    in property <string> balance-text: "0";
    in property <[BeneficiaryItem]> beneficiaries: [];

    callback beneficiary-selected(int);
    callback back-to-my-wallet-requested();
    callback help-requested();

    background: Theme.bg;

    VerticalLayout {
        padding: Theme.spacing;
        spacing: Theme.spacing;

        Text {
            text: "Семейный кошелёк";
            color: Theme.fg;
            font-size: Theme.font-size-heading;
        }

        Text {
            text: balance-text;
            color: Theme.fg;
            font-size: Theme.font-size-heading;
        }

        for b in beneficiaries: Rectangle {
            height: Theme.target-size;
            background: Theme.button-bg;
            border-radius: 8px;
            accessible-role: button;
            accessible-label: b.name;

            TouchArea {
                clicked => { beneficiary-selected(b.id); }
            }

            Text {
                text: b.name;
                color: Theme.button-fg;
                font-size: Theme.font-size-body;
                horizontal-alignment: center;
                vertical-alignment: center;
            }
        }
    }

    Rectangle {
        // Same fixed position as MyWalletScreen's "Семейный кошелёк"
        // button — spec: consistent navigation (WCAG 3.2.3/3.2.4).
        x: parent.width - self.width - Theme.spacing;
        y: Theme.spacing;
        width: 180px;
        height: Theme.target-size;
        background: Theme.secondary-button-bg;
        border-color: Theme.secondary-button-border;
        border-width: 2px;
        border-radius: 8px;
        accessible-role: button;
        accessible-label: "Назад к моему кошельку";

        TouchArea {
            clicked => { back-to-my-wallet-requested(); }
        }

        Text {
            text: "Назад к моему кошельку";
            color: Theme.secondary-button-fg;
            font-size: 20px;
            horizontal-alignment: center;
            vertical-alignment: center;
            wrap: word-wrap;
        }
    }

    Rectangle {
        x: parent.width - self.width - Theme.spacing;
        y: parent.height - self.height - Theme.spacing;
        width: Theme.target-size;
        height: Theme.target-size;
        background: Theme.secondary-button-bg;
        border-color: Theme.secondary-button-border;
        border-width: 2px;
        border-radius: 8px;
        accessible-role: button;
        accessible-label: "Помощь";

        TouchArea {
            clicked => { help-requested(); }
        }

        Text {
            text: "?";
            color: Theme.secondary-button-fg;
            font-size: Theme.font-size-heading;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }
}
```

- [ ] **Step 2: Wire it into `app.slint`**

Replace `kiosk-gui/ui/app.slint`:

```slint
import { Theme } from "theme.slint";
import { MyWalletScreen, BeneficiaryItem } from "my-wallet.slint";
import { AmountSelectScreen } from "amount-select.slint";
import { ConfirmScreen } from "confirm.slint";
import { ResultScreen } from "result.slint";
import { FamilyWalletScreen } from "family-wallet.slint";

export { BeneficiaryItem }

export enum Screen { my-wallet, amount-select, confirm, result, family-wallet }

export component AppWindow inherits Window {
    width: 800px;
    height: 480px;
    background: Theme.bg;

    in-out property <Screen> current-screen: Screen.my-wallet;

    in property <string> balance-text <=> wallet.balance-text;
    in property <[BeneficiaryItem]> beneficiaries <=> wallet.beneficiaries;
    in property <string> selected-recipient-name <=> amount-select.recipient-name;
    in property <[string]> presets <=> amount-select.presets;
    in property <string> confirm-recipient-name <=> confirm.recipient-name;
    in property <string> confirm-amount-text <=> confirm.amount-text;
    in property <bool> result-success <=> result.success;
    in property <string> result-message-text <=> result.message-text;
    in property <string> family-balance-text <=> family-wallet.balance-text;
    in property <[BeneficiaryItem]> family-beneficiaries <=> family-wallet.beneficiaries;

    callback beneficiary-selected <=> wallet.beneficiary-selected;
    callback family-wallet-requested <=> wallet.family-wallet-requested;
    callback help-requested <=> wallet.help-requested;
    callback amount-selected <=> amount-select.amount-selected;
    callback back-requested <=> amount-select.back-requested;
    callback send-confirmed <=> confirm.send-confirmed;
    callback send-cancelled <=> confirm.send-cancelled;
    callback ok-pressed <=> result.ok-pressed;
    callback family-beneficiary-selected <=> family-wallet.beneficiary-selected;
    callback back-to-my-wallet-requested <=> family-wallet.back-to-my-wallet-requested;

    wallet := MyWalletScreen {
        width: 100%;
        height: 100%;
        visible: current-screen == Screen.my-wallet;
    }

    amount-select := AmountSelectScreen {
        width: 100%;
        height: 100%;
        visible: current-screen == Screen.amount-select;
    }

    confirm := ConfirmScreen {
        width: 100%;
        height: 100%;
        visible: current-screen == Screen.confirm;
    }

    result := ResultScreen {
        width: 100%;
        height: 100%;
        visible: current-screen == Screen.result;
    }

    family-wallet := FamilyWalletScreen {
        width: 100%;
        height: 100%;
        visible: current-screen == Screen.family-wallet;
    }

    auto-return-timer := Timer {
        interval: 6s;
        running: current-screen == Screen.result;
        triggered => {
            current-screen = Screen.my-wallet;
        }
    }
}
```

- [ ] **Step 3: Drive it from Rust**

In `kiosk-gui/src/main.rs`, add after the existing `beneficiaries`/`model` setup:

```rust
    app.set_family_balance_text("500 USDC".into());
    let family_beneficiaries = vec![
        BeneficiaryItem { name: "Внук Саша".into(), id: 1 },
        BeneficiaryItem { name: "Дочка Маша".into(), id: 2 },
    ];
    let family_model = std::rc::Rc::new(slint::VecModel::from(family_beneficiaries));
    app.set_family_beneficiaries(family_model.into());
```

and replace the existing `app.on_family_wallet_requested(...)` block
(currently just a `println!`) with:

```rust
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
```

- [ ] **Step 4: Verify it builds**

Run: `cargo build --manifest-path kiosk-gui/Cargo.toml`
Expected: build succeeds

- [ ] **Step 5: Write the failing test**

Append to `kiosk-gui/tests/navigation.rs`:

```rust
#[test]
fn family_wallet_button_navigates_there_and_back_button_returns() {
    i_slint_backend_testing::init_integration_test_with_system_time();

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
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --manifest-path kiosk-gui/Cargo.toml --test navigation -- --nocapture`
Expected: all 7 tests PASS

- [ ] **Step 7: Run the full suite one final time**

Run: `cargo test --manifest-path kiosk-gui/Cargo.toml -- --nocapture`
Expected: all 7 tests pass, nothing else regressed

- [ ] **Step 8: Commit**

```bash
git add kiosk-gui
git commit -m "feat: add Family Wallet screen, mirroring My Wallet's layout for consistency"
```

---

## Out of scope for this plan (follow-up plans)

- Real Solana reads (`Vault`/`BeneficiaryList` account deserialization)
  and writes (`withdraw`/`request_withdrawal` instructions) — this plan
  is UI-only with mock data, per the design spec's own scope split.
- Local encrypted keypair signer.
- Preset-amount configuration interface for the relative (separate app,
  per the design spec).
- Alpine + cage kiosk OS image, physical 3-4 button GPIO wiring to
  Slint keyboard focus events (`backend-winit-wayland` build).
- Exact color values re-validated with a contrast-ratio calculator
  during real-hardware testing (this plan uses pure black/white, which
  is mathematically safe, but physical display gamma/glare should be
  checked once real hardware exists).
