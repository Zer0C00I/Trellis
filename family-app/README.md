# Trellis Family

The family member's app for the Trellis family vault: see the balance and recent
operations, deposit, send to approved payees, add payees and guardians, approve
over-limit transfers, and hand your key to a kiosk during its first-run setup.
A Tauri 2 shell around `tools/family-app-core`, with a dependency-free UI.

Devnet prototype, not audited. Use test money only.

## Run

Linux needs the WebKitGTK development packages (`webkit2gtk-4.1`,
`javascriptcoregtk-4.1`, `libsoup-3.0`, `gtk+-3.0`, `librsvg-2.0`, `openssl`).

```
cargo run -p family-app
```

On first launch the app creates its own key in the OS app-data directory
(`~/.local/share/dev.trellis.family/identity.json` on Linux). There is no seed
phrase and no import. The key file is not encrypted at rest in this prototype.
`TRELLIS_RPC_URL` overrides the Devnet endpoint.

A new key has no SOL, and every action costs a tiny fee: use "Получить тестовые
SOL" on the first screen (the public faucet is rate-limited; you can also send
SOL to the shown address by hand). Test USDC comes from faucet.circle.com.

## See the screens without a device or a network

The UI falls back to a scripted backend outside Tauri:

```
cd family-app/ui && python3 -m http.server 8765
# http://localhost:8765/index.html?mock=owner      (also: guardian, empty, waiting, stuck, offline, unconfirmed, unconfirmed-failed, unconfirmed-waiting, send-unconfirmed, lowsol)
```

## Tests

```
cargo test -p family-app                 # permissions, sentences, pending-operation record
cd family-app/ui && npm test             # pure JavaScript modules
cd family-app/ui && CHROMIUM=chromium npm run smoke   # every screen, headless Chromium, mock backend
```

The smoke test starts its own server and headless browser and cleans up after
itself.

## Where the rules live

- `src-tauri/src/view.rs`: who may do what (`permissions`), and whether a send is
  direct, a request above the daily limit, or refused.
- `src-tauri/src/text.rs`: every sentence the person reads, in Russian.
- `src-tauri/src/pending.rs` and `commands/mod.rs`: while an operation's outcome
  is unknown, nothing else may move money.
- One money command at a time: a shared in-flight guard (`AppState.busy`, taken
  first in every money command) refuses a second click while the first is still
  running.
- A damaged pending-operation record is moved aside to
  `pending-transaction.json.corrupt-<time>` (never deleted) and the person is
  told once to check the account history before repeating a payment.
- The UI never does money math: amounts are typed as text and parsed in Rust.

## Not done yet

Android and Windows builds, camera QR scanning, urgent add (needs two signers),
owner rotation, notifications. See `docs/superpowers/plans/2026-10-10-family-app-tauri-ui.md`.

## License

GNU Affero General Public License v3.0 only (AGPL-3.0-only). See [LICENSE](../LICENSE).
