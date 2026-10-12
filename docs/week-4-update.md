# Trellis, week 4: what was built, what the program enforces, what is still open

Written from the repository on 2026-10-12. Every rule below was read in the code; the file is named next to it.

## 1. What Trellis is

One Solana program (Anchor, Rust) holds a family's USDC in a vault tied to the owner's wallet. Three interfaces talk to that program and nothing else; no interface can do anything the program would reject.

| Interface | Stack | For |
|---|---|---|
| Web dashboard | Next.js | the vault owner: create, deposit, add family and guardians |
| Family app | Tauri 2, Rust core, plain JS UI | relatives and guardians: balance, history, send, approve, pair with a kiosk |
| Kiosk | Rust and Slint, 800x480 touchscreen | the older relative: balance, family, three taps to send |

## 2. What the program enforces

All from `programs/family-vault/src` (12 instructions: `initialize_vault`, `deposit`, `add_beneficiary`, `add_beneficiary_urgent`, `withdraw`, `initialize_approvers`, `add_trusted_approver`, `request_withdrawal`, `approve_withdrawal`, `request_owner_rotation`, `cancel_owner_rotation`, `execute_owner_rotation`).

- **Who can be paid:** only people on the vault's list, at most 10 (`MAX_BENEFICIARIES`, `state.rs`). Paying anyone else fails with `UnknownBeneficiary`.
- **48-hour wait:** a newly added person can be paid only after 48 hours (`BENEFICIARY_COOLDOWN_SECS = 172_800`). Before that the payment fails with `BeneficiaryCooldownActive`.
- **Urgent add:** `add_beneficiary_urgent` skips the wait only with two distinct approvers' signatures.
- **Daily limit (family vault):** the owner chooses `daily_limit` when creating a family ("pool") vault; it must be above zero. A send that fits in what is left today goes through at once (`withdraw`); one that would pass the limit fails with `DailyLimitExceeded` and has to become a request (`request_withdrawal`) that a trusted approver releases (`approve_withdrawal`). A request for an amount that still fits is refused (`AmountDoesNotExceedDailyLimit`), so requests are no bypass.
- **The day is a UTC calendar day** (`unix_timestamp / 86_400`): the counter resets at 00:00 UTC, not after a rolling 24 hours. Approved withdrawals count towards it.
- **Personal vault:** no limit at all (`daily_limit` is stored as `u64::MAX`) and no requests (`request_withdrawal` returns `NotPoolVault`).
- **Guardians:** at most 5 per vault (`MAX_APPROVERS`); the owner cannot be their own guardian (`add_trusted_approver`).
- **Ownership handover:** moving the vault to another key waits 72 hours (`ROTATION_TIMELOCK_SECS = 259_200`) and can be cancelled.
- **No middleman:** the vault is a token account owned by the program's address; the interfaces read straight from the chain.

## 3. Built this week

**Family app (Tauri 2).** Pairing with a kiosk over Bluetooth (QR text or six digits), balance and history, send, add a payee, add a guardian, approve a request. A pending-operation record is written before any send, so if the app dies or the network drops mid-transaction it can tell afterwards what happened instead of losing track of the money. Keys and error messages are catalogued: Rust sends a key plus parameters, the UI words them.

**English and Russian everywhere.**
- Kiosk: language chosen once at setup (`kiosk.toml`, then `provisioning.toml`, then `LANG`, then Russian). Source text is English in Slint `@tr(...)`; Russian is a `.po` catalog. Rust sends no human text.
- Family app: system language with a manual switch on the Me tab, about 190 keys per language, plural rules through `Intl.PluralRules`.
- Tests guard the catalogs: every key used exists in both languages, no key is unused, placeholders match.

**Kiosk layout fixes.** One shared button that wraps its label and grows with it (minimum 60 px), help `?` moved out of the beneficiary list, OK centred, Back no longer clipped; checked on every screen in both languages with 32-character names.

**Automated demo video.** `demo/run.sh` builds an English walkthrough with the kiosk and the family app side by side (a moving cursor, typing, finger taps, and a link column that shows what travels and over which channel). Kiosk screens are Slint snapshots; the app is the real UI on its mock backend.

**Build size.** `target/` is about half the size (debug information for dependencies removed).

## 4. Verification numbers

- `cargo test --workspace`: 350 passed, 0 failed.
- Kiosk: lib 77, navigation 15, translations 5. Family app: Rust 53, Node 39, end-to-end smoke test in headless Chromium: 26 checks in each language.
- Rejection checks: each new guard test was run against a deliberately broken input to prove it can fail.

## 5. Honest limits and open items

- **The program is not audited** and runs only on devnet with test money.
- **Demo video is scripted.** Pairing, the kiosk's request and the approval are drawn, not a live Bluetooth or chain run. Say so when presenting it.
- **No real-hardware run yet** of Bluetooth pairing against a physical kiosk, nor of the Android build and camera QR scan.
- **English text has not been read by a native speaker.**
- **The family app's identity key file is not encrypted yet.**
- **Program keypair lost.** A `cargo clean` removed `target/deploy/family_vault-keypair.json` for program `9ozR…`. The deployed program still upgrades with the upgrade authority (the wallet `417Zj…`); only a fresh deploy to that same address is no longer possible.
- **Check the 100 USDC wording.** The presentation says "at most 100 USDC can leave per day". In the code that number is not a constant: the owner picks the limit per family vault (the app's form only suggests 100 as an example). The current dashboard's create button makes a personal vault (`initializeVault(false, 0)`), which has no limit. Where 100 comes from needs confirming before judges read the source.
- **The demo's "personal account approval" scene contradicts the program** (personal vaults have no requests). The scene and the mock should be reworked so approvals happen on the family vault.
