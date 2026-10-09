# family-app Design

## Purpose

`kiosk-gui` (grandma's device) and `admin-cli` (developer's terminal
stand-in) exist. The third piece of the system — the app a family member
actually uses on their own phone/laptop to open the family account,
add payees, add trusted people, and approve large transfers — does not
exist yet. This spec designs it.

Like `kiosk-gui`, this is not a custodial app: the device generates and
holds its own Solana keypair locally, and never sends it anywhere. All
state lives on-chain in the `family-vault` program; `family-app` and
`kiosk-gui` never talk to each other directly, only through the chain
(see `docs/superpowers/specs/2026-09-21-kiosk-blockchain-wiring-design.md`
for that principle, established for kiosk and equally binding here).

## Platform & Stack

Targets: **Android, Windows, Linux (including mobile Linux distros like
postmarketOS), and best-effort Sailfish OS**. iOS is explicitly out of
scope (see "Notifications", below, for why that's the one platform this
decision actually matters for).

**Stack: Rust core + Tauri 2 shell.**

- Tauri 2 has official Android and Windows/Linux desktop targets, so one
  shell covers four of the five target platforms out of the box. Sailfish
  isn't an official Tauri target, but as a Linux/Wayland system the
  desktop build is the most likely path to work there, best-effort.
- The core (keypair handling, chain I/O, PDA derivation, instruction
  building) is Rust and reuses `family-vault-client`
  (`tools/family-vault-client`) directly — the same crate `admin-cli` and
  `kiosk-gui` were just migrated onto. No fourth reimplementation of PDA
  derivation or pubkey conversion.
- UI layer is HTML/CSS/JS inside Tauri's webview, not Slint. Slint was
  the right choice for kiosk-gui's fixed-hardware, no-text-input,
  single-purpose screen; family-app needs conventional mobile/desktop UI
  patterns (scrollable lists, text entry, multiple windows/tabs) that a
  webview-based UI serves better, and Tauri's Android support is more
  mature than Slint's for this kind of general-purpose app shell.

## Identity & Bootstrap

Same principle as kiosk: **the app generates its own keypair on first
launch, locally, and never shows or exports it as a seed phrase.** No
import-existing-wallet flow in MVP (that would reintroduce seed phrases,
which this whole project exists to avoid).

**The bootstrap problem:** a fresh keypair has no on-chain history. For
the app to show anything, that pubkey must already be listed as a
beneficiary and/or trusted approver on some vault — and `kiosk-gui`
deliberately never lets the vault owner (grandma) add people from her own
device (see the kiosk wiring spec's out-of-scope section — that
restriction exists to stop a scammer walking grandma through adding
their own address).

**Resolution: the first family member is added during the kiosk's own
first-run setup**, not from family-app's side. This is a deliberate,
narrow exception to "kiosk never adds people": at first-run time, nothing
exists yet — no vault, no beneficiaries, no approvers — and the person
physically holding and setting up the device *is* establishing the
initial chain of trust by definition (the same physical-possession trust
already implicit in "whoever funds the QR code owns the setup"). This is
categorically different from a scammer walking grandma through adding a
stranger from her *normal, everyday* wallet screen weeks later — the
attack the original restriction defends against.

Concretely, this spec requires one addition to `kiosk-gui`'s existing
first-run setup flow (`kiosk-gui/src/setup.rs`,
`docs/superpowers/specs/2026-09-23-kiosk-first-run-setup-design.md`):
after `self_initialize_personal_vault` succeeds, the setup flow receives
one family member's pubkey over **Bluetooth Low Energy** and calls
`add_beneficiary` + `add_trusted_approver` for it, signed by the device's
own owner key (which the device already holds locally at this point).
This is the *only* place in the whole system where the kiosk device
signs an "add a person" instruction. All later additions (second, third
family member) go through family-app itself, using the `add-beneficiary`
/ `add-approver` pattern already proven in `admin-cli`.

**Why BLE, not the kiosk's normal RPC network / a camera / manual
entry:** the kiosk has no camera (ruling out QR-scan-in), and typing a
44-character base58 pubkey on a touchscreen is a last resort, not a
primary path. BLE doesn't require kiosk and phone to share a WiFi
network (unlike an HTTP-over-LAN approach) — only physical proximity
(a few meters), which is itself a reasonable trust signal during a
one-time setup step nobody else should be near. Rust side, the two
roles need different crates: `btleplug` is central-only (it cannot act
as a GATT server), so family-app (central, cross-platform:
Windows/Linux/Android) uses `btleplug`, while the kiosk (peripheral, always
Linux) uses `bluer`, the official BlueZ bindings, which support
publishing GATT services and advertising. Linux-only is fine for `bluer`
because the kiosk is Linux by definition.

Mechanism: during this step, the kiosk advertises a BLE GATT service
scoped to setup only (torn down as soon as the step ends) with one
writable characteristic, and **shows a QR code on its screen**. The
family member opens family-app, **scans that QR with their phone's
camera** (the phone has a camera; the kiosk needs none), and the app
writes `secret + pubkey + name` to the characteristic. Nothing is typed.
This is the same idea as Matter commissioning, where the QR *is* the
passcode. The QR encodes a one-time random 128-bit secret
(`fvpair1:<32 hex chars>`); the kiosk accepts the write only if it carries
that secret, so a stranger in Bluetooth range cannot inject a key without
seeing the kiosk's screen.

**Fallback code.** If the phone cannot scan (broken camera, screen glare),
the kiosk also shows a short 6-digit code under the QR and family-app
offers "Ввести код вручную". The 6-digit code is a second credential the
kiosk accepts; because it is guessable, **any 5 wrong attempts make the
kiosk generate a fresh secret and code** (and redraw the QR) and start a
cooldown of 30 s, doubling with each further lockout up to a 5-minute
maximum (30, 60, 120, 240, 300, 300, ... s), during which **all attempts
are refused unevaluated** — even one carrying the new correct secret — and
do not count as wrong attempts. This bounds a nearby attacker to about 5
guesses per cooldown at a 1-in-a-million code (roughly 60 per hour at the
maximum cooldown), so brute force is impractical. Nearby garbage writes
can delay a legitimate pairing by at most 5 minutes per burst but can
never permanently prevent it: there is no permanent lockout. The screen
tells the family member to wait while a cooldown runs; the «подождите»
notice replaces the normal instruction only while the cooldown lasts and
reverts to the normal instruction when it ends, so the screen never shows a
false state. Wrong credentials /
malformed frames are answered with a proper ATT error so family-app can
say "неверный код".

The characteristic is not link-encrypted, so the QR secret crosses the air
in cleartext. That is acceptable because the secret is single-use and dead
once a request is accepted or the credentials rotate.

family-app requirements for this write: it MUST write with response
(write-with-response) so it sees the ATT error, and it MUST confirm
registration by polling the chain rather than trusting the write
acknowledgement (the kiosk may fail to submit the transaction after
acknowledging the write).

**Wire format** (single writable characteristic; shared between kiosk and
family-app via the `family-vault-client` crate so both sides use the same
encoder/decoder):

```
[ 16 bytes: auth field ]
[ 32 bytes: the family member's pubkey ]
[ 1 byte: N, the length of the name in bytes, 1..=32 ]
[ N bytes: UTF-8 name, e.g. "Внук Саша" ]
```

The auth field is either the 16-byte secret from the QR, or the 6 ASCII
digits of the fallback code zero-padded to 16 bytes; the kiosk accepts
either. Total `49 + N` bytes. A central may deliver it in one write or
split across several (ATT payloads can be as small as 20 bytes), so the
kiosk reassembles; a partial frame not completed within 3 seconds is
discarded. The name is what grandma later sees on the kiosk's "send to"
screen, which is why it travels with the key rather than being a fixed
label.

**Failure behaviour, all deliberate:**
- No Bluetooth adapter / `bluetoothd` not running: the kiosk logs it and
  skips the pairing step; setup completes and the first family member can
  be added later with `admin-cli` (so dev machines and adapter-less
  hardware are never bricked).
- A **Пропустить** button on the pairing screen skips the step for anyone
  setting the device up without the family app at hand (same fallback).
- The step runs **before** `kiosk.toml` is written and is idempotent
  (it reads the on-chain approvers/beneficiaries first and only submits
  what is missing, in one atomic transaction), so power loss mid-setup
  resumes cleanly instead of double-adding or losing the chance to pair.
- The new `TrustedApprovers` account costs rent the original funding
  floor did not include; the minimum funding amount grows accordingly.
- Note the added beneficiary is subject to the normal 48h cooldown before
  grandma can send to them; the trusted-approver role has no cooldown.

Crates: the kiosk (always Linux) is the BLE **peripheral** and uses
`bluer` (BlueZ bindings, verified to build and to register advertising +
a GATT application against a real BlueZ in this repo's environment);
family-app is the **central** and uses `btleplug`. `kiosk-gui` therefore
becomes Linux-only, which is accepted: the kiosk hardware is Linux.

## Vault Discovery

No config file points family-app at "its" vault — unlike kiosk, which is
provisioned once and always serves one specific device's own vault.
family-app instead discovers, on every launch (or refresh), every vault
its pubkey participates in or owns:

```
getProgramAccounts(program_id, [dataSize(BeneficiaryList::SPACE)])   // 1 request
getProgramAccounts(program_id, [dataSize(TrustedApprovers::SPACE)])  // 1 request
keep every list whose live entries (the first `count`) contain my_pubkey,
collect its `vault` field, dedupe
getMultipleAccounts([pool_pda(me), personal_pda(me)])                // vaults I own
```

This is architecturally multi-vault-capable: one pubkey can appear in any
number of vaults' lists and can own up to two vaults. **MVP scope decision:
the UI shows one vault at a time**; when discovery finds more than one, a
small picker lets the person switch (needed because the same person can be
a guardian on a relative's vault and also own a family account).

`getProgramAccounts` filters are ANDed, so "is my key in any of the 10
payee slots" cannot be a single filtered call, and one filtered call per
slot (15 requests) was measured to be unusable on the public Devnet node:
it allows 10 `getProgramAccounts` calls per 10 seconds per IP, so the 11th
request is answered with HTTP 429 and `retry-after: 10`, making every
launch take 20 seconds or more (parallel requests only trigger more 429s).
Discovery therefore asks for every list of one kind, filtered only by
exact account size (2 requests, about 135 ms), and checks for the key on
the client. Each list is 851 (payees) or 201 (guardians) bytes, so this is
fine at MVP scale (a thousand vaults is about a megabyte). A deployment
with many vaults should put an indexer, or a provider that supports
server-side filtering, in front of the same two requests.

## MVP Scope

Full parity with `admin-cli`'s everyday command set, translated into
plain bank language — no "vault", "PDA", "mint", "approver" anywhere in
the UI:

| Screen / action | admin-cli equivalent |
|---|---|
| Открыть семейный счёт | `create-vault --pool` |
| Пополнить | `deposit` |
| Отправить | `send` / `withdraw` |
| Добавить получателя | `add-beneficiary` |
| Добавить получателя срочно | `add-beneficiary-urgent` (needs 2 distinct trusted people) |
| Доверенные лица (список + добавление) | `add-approver` |
| Одобрить перевод | `approve-withdrawal` (needed once a transfer exceeds the pool vault's daily limit) |
| История операций | read-only: decode past instructions/events for this vault |

Every one of these already has a working, devnet-verified instruction
path in `family-vault` and a reference implementation in `admin-cli` —
family-app's job is a UI and identity layer on top, not new on-chain
logic.

## Screens

1. **First launch** — generate keypair locally → during a kiosk's own
   first-run setup, scan the QR the kiosk shows (or type its fallback
   6-digit code) and enter your name; the app hands the pubkey over via
   BLE (see "Identity & Bootstrap") → "ждём, пока вас добавят" status,
   matching the plain-language, no-jargon tone kiosk already uses for its
   own first-run screen.
2. **Discovery** — silent on every launch/refresh; once a vault is found,
   go straight to the main screen. No user-visible "searching" step
   needed if it completes in normal RPC latency; if it doesn't, reuse the
   same "Нет связи" / "Не получилось" states as kiosk's `ChainError`.
3. **Счёт** (main/home) — balance, recent history, buttons for the
   actions above.
4. **Перевод** — amount, pick recipient from the payee list (never a
   free-text address field — same closed-address-book principle as
   kiosk).
5. **Доверенные лица** — list, add (normal or urgent-two-approver path).
6. **Одобрения** — list of pending withdrawals awaiting this person's
   signature, if any.

## Data Flow & Error Handling

Same polling pattern as kiosk: periodic refresh (interval TBD in the
plan — kiosk uses 30s) plus refresh-on-return-to-foreground, no push, no
websocket. Reuses `kiosk-gui/src/chain.rs`'s two-state `ChainError` model
(`NoConnection` vs `ProgramRejected`) directly rather than inventing a
third error taxonomy — extract it to `family-vault-client` if/when the
plan needs it in both crates without a `kiosk-gui` dependency, since
`family-app` shouldn't need to depend on the kiosk binary crate for an
error enum.

## Notifications — explicitly out of scope for this spec

Discussed at length in this session: real-time push notifications
require either a background service (viable on Android/Linux/Windows,
architecturally impossible on iOS without a push relay server) or the
user simply opening the app to see new state. Since iOS is out of scope
for this app entirely, MVP relies on **open-the-app-to-see-it** polling.
A background watcher/notification service remains a possible future
addition (a separate component, not touched by this spec) if
"approval needed" latency becomes a real problem — not needed to ship a
working MVP.

## Explicitly Out of Scope (MVP)

- Multi-vault switcher UI (architecture supports it; UI doesn't in MVP).
- Push notifications / background watcher service.
- Importing an existing wallet / seed phrase (contradicts the project's
  core no-seed-phrase principle).
- Owner-rotation flows (`request-rotation`/`execute-rotation`) — rare,
  high-stakes, and not part of the everyday "how does a family use this"
  loop; can be added later using the same pattern as everything else
  here.
- Sailfish OS as a fully-supported target (best-effort only, not tested
  as part of this plan's acceptance criteria).

## Testing

Same standard as the rest of the workspace: `family-vault`'s on-chain
logic is already covered by its own litesvm tests — this spec adds no
new on-chain behavior, so no new program tests. New Rust core logic
(discovery-query construction, error mapping, any new pure functions)
gets unit tests the same way `family-vault-client` and `kiosk-gui`
already do. UI/webview behavior is verified manually (Tauri's webview
testing story is less mature than `kiosk-gui`'s Slint test harness) —
this spec accepts that gap rather than inventing new test infrastructure
under a hackathon deadline; call this out explicitly in the plan.
