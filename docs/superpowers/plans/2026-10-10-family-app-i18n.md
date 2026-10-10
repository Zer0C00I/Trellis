# Family App i18n Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The family app speaks English or Russian. It follows the system language on first launch, the person can switch on the "Me" tab, and the choice is remembered. No sentence is built in Rust any more.

**Architecture:** Rust returns `{ key, params }` (a `Msg`) for every sentence; params may themselves be messages (an error that names the operation it blocks). One catalog per language in `family-app/ui/locales/` and a DOM-free module `ui/i18n.js` (`t`, `tMsg`, `tParts`, `Intl.PluralRules` for plurals, `Intl.DateTimeFormat` for dates) turn them into text. The mock backend returns the same keys, so every screen runs in a plain browser and under the headless smoke test, which now runs the whole suite in both languages.

**Tech Stack:** Rust (Tauri 2 app crate `family-app`, `serde_json`), vanilla JavaScript ES modules, `node --test`, headless Chromium over the DevTools protocol (existing smoke test).

**Spec:** `docs/superpowers/specs/2026-10-10-i18n-design.md`, section 2 (family app). The kiosk half is a separate plan (`2026-10-10-kiosk-i18n.md`).

## Global Constraints

- **Languages:** `ru` and `en`. Language order of choice: saved choice (`localStorage` key `trellis.locale`), else the prefix of `navigator.language` (`ru`/`en`), else `en`. A demo page may force one with `?lang=ru|en` in mock mode only (never inside the real app, like `?mock`).
- **Rust sends no human text.** Every sentence is a catalog key plus params. Amounts are formatted by Rust (`money`, e.g. `"5 USDC"`), names are cleaned by `sanitize_text`; JavaScript puts them in as they are. JavaScript still never parses or compares amounts.
- **Catalogs are plain JS modules** (`export default { 'key': 'text' }`), not JSON: the app's CSP (`connect-src ipc: http://ipc.localhost`) does not allow `fetch` of its own files, and a script import is allowed.
- **No untranslated fallbacks in tests:** a missing key is a test failure (the `hooks.onMissing` hook), and the smoke test treats an `[i18n]` console warning as an error.
- **Plain bank language in both languages:** never show "vault", "PDA", "mint", "approver", "beneficiary" or "nonce" to the person (the English catalog says "account", "payee", "guardian").
- **Untrusted text:** names and addresses come from the chain; the UI builds DOM with text nodes only (existing `dom.js`).
- **Rust:** `unsafe_code = "deny"`, `dead_code = "deny"`; `cargo fmt -p family-app --check` and `cargo clippy -p family-app --all-targets -- -D warnings` clean.
- **Commits:** work directly on `master`. Do not add attribution lines to commit messages. Never `git add` anything under `target/`, `node_modules/` or `family-app/src-tauri/gen/`. No task changes `Cargo.lock`.
- **The app is not runnable between Task 2 and Task 3** (Rust already sends keys, the UI still expects sentences). Do not stop there; if work is split across sessions, finish Task 3 first.

## How the code is delivered

Every task ships as a patch in `docs/superpowers/plans/family-app-i18n/`. They were produced by implementing the tasks in order on a clean export of `master` at `0be2aac`, then re-applied in order to a fresh export of `master` where fmt, clippy, `cargo test -p family-app`, `npm test` and the smoke test all passed. Apply each with `git apply --index <patch>` from the repo root. If a patch does not apply, `master` has moved: read the patch (a plain unified diff, the specification of the change) and make the same change by hand.

## File Structure

| File | Responsibility |
|---|---|
| `family-app/ui/i18n.js` (new) | `t`, `tMsg`, `tParts`, `setLocale`, `initLocale`, `detectLocale`, `currentLocale`, plural pick, nested-message params |
| `family-app/ui/locales/en.js`, `ru.js` (new) | the catalogs (about 190 keys each) |
| `family-app/ui/format.js` | dates via `Intl.DateTimeFormat(locale)` and catalog templates |
| `family-app/ui/errors.js` | normalizes an `ErrorDto` of the new shape `{ kind, key, params, details? }` |
| `family-app/ui/components.js`, `app.js`, `screens/*.js` | every string goes through `t()`; `rich()` keeps amounts unbroken |
| `family-app/ui/mock-api.js` | returns keys and params, same shapes as Rust |
| `family-app/ui/screens/me.js` | the language switch |
| `family-app/ui/tools/smoke.mjs` | the suite, run in both languages, plus switching and 360 px overflow checks |
| `family-app/ui/test/{i18n,catalog}.test.js` (new) | catalog parity, parameters, plurals, key usage |
| `family-app/src-tauri/src/msg.rs` (new) | the `Msg { key, params }` type |
| `family-app/src-tauri/src/{dto,text,view,pending}.rs`, `commands/*.rs` | DTOs and every sentence become keys and params |

## Catalog key sections

`tabs`, `nav`, `header`, `vault`, `common`, `pending`, `first`, `pair`, `waiting`, `open`, `address`, `faucet`, `home`, `deposit`, `time`, `send`, `people`, `approvals`, `me` (screens, written by JS) and `error` (including `error.vault.*` and `error.pairing.*`), `field`, `notice`, `op`, `payee`, `history`, `resolution` (sentences Rust sends). A plural entry is an object with the language's forms (`en`: `one`, `other`; `ru`: `one`, `few`, `many`, `other`); the form is chosen by `params.n`.

---

### Task 1: Translation module and catalogs

**Files:**
- Create: `family-app/ui/i18n.js`, `family-app/ui/locales/en.js`, `family-app/ui/locales/ru.js`, `family-app/ui/test/i18n.test.js`
- Modify: `family-app/ui/format.js`, `family-app/ui/test/pure.test.js`
- Patch: `docs/superpowers/plans/family-app-i18n/1-catalogs.patch`

**Interfaces:**
- Produces: `i18n.js` exports `LOCALES = ['en','ru']`, `currentLocale()`, `setLocale(locale, { persist = true })` (unknown locales ignored; also sets `document.documentElement.lang`), `detectLocale({ stored, navigatorLanguage })`, `initLocale()` (reads storage and `navigator`, never throws), `t(key, params = {})`, `tMsg({ key, params })`, `tParts(key, params)` (array of `{ text }` / `{ param, text }`), `catalog(name)`, `hooks.onMissing(key)`. A param whose value is `{ key, params }` is translated in turn. `format.js`: `formatTime(unixSeconds, nowMs, timeZone, locale = currentLocale())`.

- [ ] **Step 1: Apply the patch**

```bash
git apply --index docs/superpowers/plans/family-app-i18n/1-catalogs.patch
```

`format.js` no longer holds Russian month names: it asks `Intl.DateTimeFormat(locale, { day, month: 'short' })` and assembles the result from the catalog templates `time.today`, `time.this_year` and `time.other_year` (`"Dec 31, 2025, 23:59"` in English, `"31 дек. 2025, 23:59"` in Russian).

- [ ] **Step 2: Run the tests**

Run: `cd family-app/ui && npm test`
Expected: 34 tests, all pass. New: `i18n.test.js` (locale detection and storage, parameters, nested messages, plurals for 1/2/5/11/21/48 in Russian and 1/2/48 in English, parts, missing key, catalogs have identical keys and identical parameters per key, plural entries have every form their language needs); `pure.test.js` checks dates in both languages.

- [ ] **Step 3: Prove the parity tests are not vacuous**

Delete one line from `locales/ru.js` (any key), run `npm test`: `both catalogs have the same keys` must fail. Then in `locales/en.js` change one `{name}` placeholder to `{nam}` in a key that has parameters: `every key has the same parameters in both languages` must fail. Restore both files.

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(family-app): translation module and the English and Russian catalogs"
```

---

### Task 2: Rust sends keys and parameters

**Files:**
- Create: `family-app/src-tauri/src/msg.rs`
- Modify: `family-app/src-tauri/src/{dto,text,view,pending,lib}.rs`, `family-app/src-tauri/src/commands/{mod,actions,read}.rs`
- Patch: `docs/superpowers/plans/family-app-i18n/2-rust-keys.patch`

**Interfaces:**
- Consumes: the catalog key names from Task 1 (`error.*`, `notice.*`, `op.*`, `payee.*`, `history.*`, `resolution.*`, `field.*`).
- Produces: `msg::Msg { key: String, params: serde_json::Map<String, Value> }` with `Msg::new(key)`, `.with(name, value)`, `.with_msg(name, &Msg)`. DTOs: `ErrorDto { kind, #[serde(flatten)] message: Msg, details }` (`ErrorDto::new(kind, key)`, `from_msg`, `with_msg_param`, `with_details`); `ActionOkDto { signature, message: Msg flattened }`; `HistoryLineDto { signature, text: Msg flattened, time }`; `ResolutionDto { status, message: Msg flattened }`; `PayeeDto.status: Msg`; `PendingTxDto { what: Msg, signature }`; `VaultDto` loses `title`. `PendingTx { signature, last_valid_block_height, what_key: Option<String>, what_params, what: String }` with `PendingTx::new(signature, height, &Msg)` and `what_msg()`. `commands::run_money(state, what: &Msg, op)`; `parse_pubkey(text, field_key)`; `finish(result, notice_key)`; `not_allowed(key)`.

- [ ] **Step 1: Apply the patch**

```bash
git apply --index docs/superpowers/plans/family-app-i18n/2-rust-keys.patch
```

Notes on what it does:
- `text.rs` loses `plural_ru` and every Russian literal; `payee_status` returns `payee.ready`, `payee.wait_under_hour` or `payee.wait_hours` with `n` (hours, rounded up); `history_line` returns `history.*` with `amount` or a cleaned `name`; `friendly_rejection` returns the key `error.vault.<snake_case variant>` or `None`.
- **Pending record compatibility:** new records are written with `what_key`/`what_params` and an empty `what`, so an older build can still parse them; a record made by an older build (only `what`) is shown as `op.legacy` with its old sentence as the `text` param. Both directions have a test.
- `commands/mod.rs::ensure_no_pending` returns `error.pending_blocks` whose `what` param is the pending operation's own message (nested).

- [ ] **Step 2: Check no Russian remains in non-test Rust**

Run: `for f in family-app/src-tauri/src/*.rs family-app/src-tauri/src/commands/*.rs; do awk '/#\[cfg\(test\)\]/{t=1} !t && /[А-Яа-яЁё]/ && $0 !~ /^\s*\/\//{print FILENAME": "$0}' $f; done`
Expected: no output.

- [ ] **Step 3: Run the checks**

```bash
cargo fmt -p family-app --check
cargo clippy -p family-app --all-targets -- -D warnings
cargo test -p family-app
```

Expected: fmt and clippy clean; 53 tests pass (46 before; the plural helper's test is gone, and `msg`, `pending`, `commands` and `text` gained tests). `msg::tests::every_key_the_app_sends_is_in_both_catalogs` scans every `.rs` source for key-shaped string literals (sections `error.`, `notice.`, `op.`, `payee.`, `history.`, `resolution.`, `field.`) and checks both `ui/locales/en.js` and `ru.js` contain each.

- [ ] **Step 4: Prove the coverage test is not vacuous**

Delete the line for `"error.busy"` from `family-app/ui/locales/en.js`, run `cargo test -p family-app`: the test must fail with `en catalog lacks error.busy`. Restore the file.

- [ ] **Step 5: Commit**

```bash
git commit -m "refactor(family-app): Rust sends catalog keys and parameters instead of Russian sentences"
```

---

### Task 3: Screens, mock backend and smoke test use the catalogs

**Files:**
- Modify: `family-app/ui/{app,components,errors,mock-api,state}.js`, `family-app/ui/index.html`, `family-app/ui/screens/{approvals,first,home,me,people,send}.js`, `family-app/ui/locales/{en,ru}.js`, `family-app/ui/tools/smoke.mjs`, `family-app/ui/test/{hardening,pure}.test.js`
- Create: `family-app/ui/test/catalog.test.js`
- Patch: `docs/superpowers/plans/family-app-i18n/3-ui.patch`

**Interfaces:**
- Consumes: `t`, `tMsg`, `tParts`, `initLocale`, `setLocale` from Task 1; the DTO shapes from Task 2.
- Produces: `components.js::rich(key, params, keepTogether = [])` (a translated sentence as DOM children, with the named params in `nowrap` spans); `ctx.notify(message, kind)` now takes `{ key, params }` (a DTO from Rust works as is) and the notice is worded again on re-render; `errors.js::normalizeError` returns `{ kind, key, params, details }` (an old-shaped error without a `key` becomes `error.generic` with the original text as details).

- [ ] **Step 1: Apply the patch**

```bash
git apply --index docs/superpowers/plans/family-app-i18n/3-ui.patch
```

What changes: no Russian is left in the screens (grep below). Tab labels, vault picker, banners, forms, placeholders and hints all use `t()`. `app.js` calls `initLocale()` at start and, in mock mode only, honours `?lang=`. `mock-api.js` returns the same keys and params as Rust (payee status, history lines, notices, resolutions, errors). `smoke.mjs` defines its tests in a function and runs them for `ru` and then `en`; expected text comes from `t()` (imported from `../i18n.js`), so nothing is duplicated; an `[i18n]` console warning fails the run. `index.html` starts as `lang="en"` (`setLocale` keeps it in step). `locales/*.js` loses the then-unused `me.language`.

- [ ] **Step 2: Check no Russian remains in the screens**

Run: `grep -rn "[А-Яа-яЁё]" family-app/ui --include=*.js --include=*.html --exclude-dir=node_modules --exclude-dir=locales --exclude-dir=test --exclude-dir=tools`
Expected: only `mock-api.js` lines with Russian *names* of people (`Внучка Анна`, `Сосед Пётр`) — user data, not interface text.

- [ ] **Step 3: Run the tests**

```bash
cd family-app/ui && npm test
CHROMIUM=chromium node tools/smoke.mjs
```

Expected: `npm test`: 39 tests pass (adds `catalog.test.js`: every key used by `ui/**/*.js` exists in both catalogs; every key Rust sends exists; no catalog key is unused). Smoke: `ok` for all 23 tests under `[ru]` and again under `[en]`, then `all passed`. If Chromium has another name, set `CHROMIUM=/path`.

- [ ] **Step 4: Prove the usage test is not vacuous**

In `screens/me.js` change `'me.copied'` to `'me.copiedx'`, run `npm test`: both `every key the screens use is in the catalog` and `no catalog key is left unused` must fail. Restore the file.

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(family-app): screens, mock backend and smoke test use the catalogs; smoke runs in both languages"
```

---

### Task 4: Language switch on the Me tab

**Files:**
- Modify: `family-app/ui/app.js`, `family-app/ui/screens/me.js`, `family-app/ui/tools/smoke.mjs`
- Patch: `docs/superpowers/plans/family-app-i18n/4-language-switch.patch`

**Interfaces:**
- Consumes: `setLocale`, `LOCALES`, `currentLocale` from Task 1.
- Produces: `ctx.setLanguage(code)` (sets and saves the locale, re-renders everything).

- [ ] **Step 1: Apply the patch**

```bash
git apply --index docs/superpowers/plans/family-app-i18n/4-language-switch.patch
```

The "Me" tab gets a card titled `Language / Язык` (a fixed bilingual title, so a person in the wrong language can still find it) with two buttons named in their own language, `English` and `Русский`; the current one has `aria-pressed="true"` and is the filled style.

- [ ] **Step 2: Run the tests**

```bash
cd family-app/ui && npm test && CHROMIUM=chromium node tools/smoke.mjs
```

Expected: `npm test` 39 pass; smoke: every test `ok` in both languages, `all passed`. The three new smoke tests (per language): switching on the Me tab rewords the page, sets `<html lang>`, is remembered across a reload without `?lang` (and clears `localStorage` afterwards); a notice already on screen is reworded when the language changes; no screen of the owner, guardian, first-launch, waiting and unconfirmed scenarios is wider than a 360 px phone (`scrollWidth <= clientWidth` and no element's right edge beyond the viewport).

- [ ] **Step 3: Prove the overflow test is not vacuous**

Append `.card{min-width:420px}` to `family-app/ui/styles.css` and run the smoke test: `no screen is wider than a 360px phone` must fail in both languages with a `scroll` larger than `width`. Restore the file.

- [ ] **Step 4: Commit**

```bash
git commit -m "feat(family-app): language switch on the Me tab; smoke checks switching and 360px overflow"
```

---

## Final checks (after Task 4)

- [ ] `cargo fmt -p family-app --check`, `cargo clippy -p family-app --all-targets -- -D warnings`, `cargo test -p family-app` (53 tests) pass; `git diff --stat Cargo.lock` is empty.
- [ ] `cd family-app/ui && npm test` (39) and `CHROMIUM=chromium node tools/smoke.mjs` (`all passed`) pass.
- [ ] Open the real app once (`cargo tauri dev` or the existing run script) with `LANG=en_US.UTF-8` and with `LANG=ru_RU.UTF-8`, and switch language on the Me tab. This needs a display and is the one check left for a human; nothing here touches the chain logic.
- [ ] Update `family-app/README.md` if it still says every sentence is Russian (it should say the app is English/Russian, language from the system or the Me tab).

## Self-review against the spec

| Spec item (section 2) | Task |
|---|---|
| 2.1 catalogs, `t`, `setLocale`/`currentLocale`/`detectLocale`, storage key `trellis.locale`, `Intl` dates and plurals | 1 |
| 2.1 switch on the Me tab, re-render without restart | 4 |
| 2.2 DTO changes (`ErrorDto`, `ActionOkDto`, `PayeeDto.status`, `HistoryLineDto`, `ResolutionDto`, `VaultDto.title` removed, `PendingTxDto`) | 2 |
| 2.2 `plural_ru` removed, `short_address` kept, `friendly_rejection` returns keys | 2 |
| 2.2 pending-record backward compatibility | 2 |
| 2.3 all UI strings replaced, mock returns keys, smoke in both languages (`?lang=`), overflow check at 360 px | 3, 4 |
| 2.4 catalog tests (same keys, same parameters, plural forms, keys used / not unused, Rust keys exist) | 1, 2, 3 |

Deviations from the spec, each forced by something verified while preparing the plan, and recorded in the spec: catalogs are `.js` modules, not JSON (the CSP forbids `fetch` of own files); `PendingTxDto` carries `what: { key, params }` instead of `what_key` and `what_params` (so it can be passed to `t()` as a nested param); a legacy pending record shows as `op.legacy` with its old sentence rather than a generic text with the sentence in details; the "Language / Русский" title is fixed text, not a catalog key.

Known limits: the person-visible English strings were written for this plan and have not been reviewed by a native speaker; the Rust key-coverage test and the Node usage test find keys by scanning for string literals, so a key built by string concatenation would not be seen (none exist; keep it that way).
