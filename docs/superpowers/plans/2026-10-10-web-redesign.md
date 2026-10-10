# Trellis frontends: visual and UX redesign

Scope: `web/` only (Next.js marketing site and owner dashboard).
Out of scope: `programs/`, `tools/`, `kiosk-gui/`, `family-app/`, the IDL, `src/config/solana.ts`, `src/config/useAnchorProvider.ts`, and every on-chain call (method names, accounts, seeds, amounts). Handlers stay as they are. Only their markup, copy, and the state they show around them change.
Nothing is committed or pushed. You review on `localhost:3000` first.

## 1. What's there now

**Subject:** self-custodial crypto inheritance on Solana. An owner puts USDC in a vault, names family members who can receive it (48 h wait before a new name can be paid), sets a daily limit (100 USDC), and adds guardians who must sign anything above it.
**Audience:** people who want their family covered without a lawyer or a bank, plus Colosseum judges watching a 2-minute demo.
**Primary job:** make the rules clear in one screen, then get to the dashboard demo (connect Phantom, create vault, add family, show the limit).

Problems found:

| Area | Issue |
| --- | --- |
| Look | Generic web3 kit: black and navy with blue/emerald blur glows, Inter, rounded-3xl cards with the same shadow, purple gradient wallet button. Nothing ties it to "Trellis" or to family. |
| Template tells | `[ BLOCK: 849201A3F ] CORE CAPABILITIES` mono all-caps eyebrows on every section, `&rarr;` on every link, emoji status badges (🟢 🟡), fade-up on every section. |
| Consistency | Sections jump between black, white, `#050505` and `blue-950`. The dashboard uses zinc, the landing page uses slate, the theme tokens use `#0a0f1e`. The nav is copy-pasted into 5 pages. |
| Logo | Black-on-white PNG with heavy padding, forced to work through `invert mix-blend-screen` and negative margins (`-my-6`, `-my-12 -ml-8`). |
| Dashboard | Three identical stat cards and a table. Guardians are never listed after you add them. Pending withdrawals only appear as a count. No "what's next" after creating a vault. The not-connected state is one line of text with no button. |
| Forms | No inline validation. Errors only show as toasts. You find out the name is over 32 bytes after you press submit. No "Max" on deposit, and your wallet balance isn't shown. |
| Plumbing | Two `<Toaster>` instances (layout plus page). A cluster dropdown on a Devnet-only app. Fonts loaded with a CSS `@import`. |

## 2. Design direction

A trellis is a lattice that holds up something growing. Wisteria is the classic plant grown on one. The site takes its look from that: a fixed structure (the rules the program enforces) and something that grows on it (what the family receives). Each rule is drawn as a crossing in the lattice.

### Color

| Token | Hex | Role |
| --- | --- | --- |
| Lichen | `#E9ECE4` | Page ground. A cool grey-green paper, not cream. |
| Ivy | `#17301F` | Ink, headlines, dark sections (the kiosk). |
| Moss | `#5B6B5F` | Secondary text, hairlines (at 20–30%). |
| Wisteria | `#5B4AA0` | The only action color: primary buttons, links, focus ring. 6.0:1 on Lichen, 7.2:1 with white text. Also a quiet nod to Solana purple. |
| Bloom | `#D9D1F0` | Tints: selection, active row, blossoms in the hero. |
| Status set | Sap `#3E7A4F` (can receive), Amber `#A86A12` (waiting), Rust `#A3402E` (error). | Used only for status, never as decoration. |

One theme, light. The kiosk section and the dashboard's "vault" panel use Ivy as a surface, so the dark color has a job: it marks a vault or a device. No blur glows, no gradients.

### Type

- **Onest** for everything (see section 6 for why not Familjen Grotesk). A warm grotesk built for Russian public services, legible for older readers, with Cyrillic for the two apps. Headlines 600 with tight tracking. Body 400 at 17–18 px, line-height 1.55, measure ≤ 68ch.
- **JetBrains Mono** for wallet addresses and transaction signatures only, where telling `l` from `1` matters. Not for labels.
- Amounts use tabular figures. Scale is 1.25 (minor third): 14 / 17 / 21 / 27 / 34 / 42 / 53 / 66 / 84.
- Sentence case everywhere. No eyebrows above headings, no all-caps labels.

### Shape and structure

- Radius by rank: 6 px for controls and rows, 14 px for panels, 999 px for status dots and nothing else.
- Borders are Moss at 25%. One soft shadow, for floating layers only (dialogs, wallet menu).
- Numbers appear only where the content is a real sequence: the four setup steps.

### Motion

One planned moment: on load, the hero lattice draws itself (stroke-dashoffset, about 0.9 s), then a wisteria vine grows across it and three blossoms open, each labelled with a rule. With `prefers-reduced-motion`, it renders already finished. Everything else only moves when the user acts: the allowance bar fills when spend changes, setup steps tick when completed, dialogs open. The per-section `whileInView` fades are removed.

### Layout sketches

Landing, desktop:

```
┌──────────────────────────────────────────────────────────────────────┐
│ Trellis(script)        How it works  Kiosk  Pricing  Docs  [Open app] │
├──────────────────────────────────────────────────────────────────────┤
│ Secure crypto inheritance          │   ╲ ╱ ╲ ╱ ╲ ╱  lattice           │
│ without middlemen.                 │    ╳ ✿ ╳   ╳   ✿ "48 h before a  │
│                                    │   ╱ ╲ ╱ ╲ ╱ ╲     new name is    │
│ Your USDC stays in a vault only    │    ╳   ╳ ✿ ╳      paid"          │
│ you control. You choose who it     │   vine grows ───────►            │
│ goes to, how much can leave each   │                                  │
│ day, and who signs off on more.    │                                  │
│ [Create a vault] [See the rules]   │                                  │
├──────────────────────────────────────────────────────────────────────┤
│ Setting up takes four steps        1 Create  2 Name family  3 Fund   │
│                                    4 Add a guardian   (real sequence)│
├──────────────────────────────────────────────────────────────────────┤
│ What the program enforces    (a schedule, like the back of a deed)   │
│   Family members per vault ............................... 10        │
│   Wait before a new name can be paid ..................... 48 h      │
│   Daily limit, resets 00:00 UTC .......................... 100 USDC  │
│   Guardians ............................................... up to 5  │
│   Signatures to fast-track a name ......................... 2        │
│   Wait before ownership changes hands ..................... 72 h     │
├──────────────────────────────────────────────────────────────────────┤
│ ▓▓ Ivy section: the kiosk ▓▓  a drawn device showing the real kiosk  │
│    screen (balance, "Send to family")  │  copy about no seed phrase   │
├──────────────────────────────────────────────────────────────────────┤
│ Pricing as one comparison table (Personal / Pro / Advisor / Terminal)│
│ Questions (plain disclosure list)                                    │
│ Footer                                                               │
└──────────────────────────────────────────────────────────────────────┘
```

Left-aligned throughout. Text is not centered except inside small buttons. Mobile: the hero lattice moves under the copy at 60% height, and the schedule rows wrap value-under-label.

Dashboard, desktop (owner view):

```
┌───────────────────────────────────────────────────────────────────────┐
│ Trellis   Vault  Docs                     ● Devnet   [7xKp…3f2a ▾]     │
├───────────────────────────────────────────────────────────────────────┤
│ ┌─ Ivy panel ───────────────────┐  Setup  ✓ Vault  ○ Fund  ○ Family   │
│ │ Vault balance                 │         ○ Guardian   (hides when    │
│ │ $1,240.00 USDC                │          all four are done)         │
│ │ Today  ███████░░░  64 of 100  │                                     │
│ │ Resets in 5 h 12 m            │  Waiting for a guardian (2)         │
│ │ [Deposit]   vault 9ozR…4zwJ ⧉ │   40 USDC to Ana · asked 3 h ago    │
│ └───────────────────────────────┘                                     │
│                                                                       │
│ Family                                             [Add family member]│
│   Ana (daughter)    Fq3…k9P ⧉   ● Can receive now                    │
│   Deniz             2bH…uW1 ⧉   ◔ Can receive in 31 h                │
│                                                                       │
│ Guardians                                              [Add guardian] │
│   Hk2…Lq8 ⧉   added   (owner can never be a guardian)                │
└───────────────────────────────────────────────────────────────────────┘
```

The vault panel is the one bold block. Family and guardians are quiet lists with hairline rows, not cards.

## 3. Checked against the brief

Asked of a generic "crypto inheritance site", this plan would have come out as: dark navy, purple/teal glow, three feature cards, serif gravitas. Changes made after that check:

1. **Serif display → a warm grotesk (now Onest).** My first idea was a high-contrast serif for "estate" weight. That is the cream-plus-serif cliché. The grotesk carries warmth through letterforms instead.
2. **Mono for amounts → tabular figures.** Mono is now only for addresses, where it does real work.
3. **Feature cards → a rules schedule.** The product *is* its rules, so they are shown as a list of numbers you can check against the program, not three paragraphs in boxes.
4. **01/02/03 everywhere → numbers only on the setup steps.** That is the one true sequence.
5. **Dark green ground + bright accent → light Lichen ground, Ivy only as a surface.** This avoids the near-black plus single accent look.

## 4. Work, in order

Each step leaves the site runnable. I screenshot desktop and 375 px after each one and review before moving on.

1. **Setup.** Run `npm install`. Read the relevant `node_modules/next/dist/docs/` pages, as `web/AGENTS.md` requires (fonts, layouts, static export). Run once to screenshot the current state as a baseline.
2. **Tokens.** Rewrite `globals.css`: new palette mapped onto the existing shadcn variable names (so `ui/*` components keep working), radius scale, focus ring, `::selection`, and a wallet-adapter button and modal restyle. Load fonts with `next/font/google` in `layout.tsx` and drop the CSS `@import`. Remove the forced `className="dark"`.
3. **Logo.** Make a tightly cropped `logo-mark.png` from `public/logo.png` (script only, transparent background) using `sips`. Show it with `mix-blend-multiply` on Lichen and inverted on Ivy. The original file is kept.
4. **Shared chrome.** Add `components/site/site-header.tsx` and `site-footer.tsx`, used by all five marketing pages, replacing the 5 copied navs. Include a mobile menu (it doesn't exist on the marketing pages today), a "skip to content" link, and visible focus.
5. **Hero lattice.** `components/site/lattice-hero.tsx`: inline SVG lattice, vine path, three labelled blossoms, CSS-only draw-in, reduced-motion fallback. The rule labels come from constants (`DEFAULT_POOL_DAILY_LIMIT_USDC` etc.), so they can't drift from the program.
6. **Landing page.** Rebuild `app/page.tsx` to the sketch: hero, four steps, rules schedule, kiosk section (CSS-drawn device copying the real `kiosk-gui` screens), pricing table, questions, footer. Copy keeps the pitch line ("Secure crypto inheritance without middlemen.") and the facts already on the page, minus the template chrome.
7. **Subpages.** Restyle `features`, `hardware`, `pricing`, `docs` in the same system. Docs keeps its sidebar layout, restyled, with the code block on an Ivy surface. Content is unchanged except removing `[ BLOCK ]` labels and arrows.
8. **Dashboard shell.** New `AppLayout`/`AppHeader`: same nav links, the cluster dropdown restyled (kept, per decision B), a restyled wallet button, and one `<Toaster>` for the whole app.
9. **Dashboard states.**
   - *Not connected:* a short explanation plus a real "Connect wallet" button (opens the wallet-adapter modal).
   - *Loading:* a skeleton in the final layout, not a spinner.
   - *No vault:* a three-item checklist (wallet on Devnet ✓, at least 0.02 SOL with the existing airdrop button, Create vault). Same handlers and same button ids.
   - *Vault:* the layout above. Vault panel with balance, today's allowance bar, and a "resets in" countdown worked out in the browser from UTC midnight. A setup checklist derived from data already loaded (balance > 0, family count, guardian count). Family list with a cooldown ring and an "in 31 h" label from the existing `activeAfter`. A **guardians list** and **pending withdrawals list**, both read-only fetches of accounts the IDL already defines (`trustedApprovers`, `pendingWithdrawal`). Copy buttons for addresses, explorer links for the vault.
10. **Dialogs.** Restyle the three modals. Add inline validation that runs as you type: Solana address check, name byte counter `n/32`, amount format, wallet USDC balance plus a "Max" button in Deposit. Errors appear under the field. The submit handlers and their transactions are untouched. Rewrite copy so button and toast match ("Add family member" → "Family member added").
11. **Quality pass.** Keyboard tab-through on every page, contrast check of all token pairs, 375/768/1280 screenshots, `npm run build` (static export must still work), `npm run lint`, `npm run format:check`.
12. **Hand-off.** Leave the changes uncommitted and give you a file-by-file summary and the `npm run dev` command.

## 5. Files

Changed: `src/app/globals.css`, `src/app/layout.tsx`, `src/app/page.tsx`, `src/app/{features,hardware,pricing,docs}/page.tsx`, `src/app/dashboard/{layout,page}.tsx`, `src/components/app-{layout,header,footer}.tsx`, `src/components/dashboard/{BeneficiaryTable,AddBeneficiaryModal,AddApproverModal,DepositUsdcModal}.tsx`, `src/components/solana/wallet-button.tsx`, `src/components/ui/{button,input,dialog,badge,table}.tsx` (styles only).
New: `src/components/site/*`, `src/components/dashboard/{VaultPanel,SetupChecklist,GuardianList,PendingList,AddressChip}.tsx`, `public/logo-mark.png`.
Not touched: everything outside `web/`, plus `web/src/config/*`, `web/src/idl/*`, `web/src/lib/idl/*`, `web/src/hooks/*`, `web/family-vault-portal/*`.

## 6. Decisions (answered 2026-10-10)

- **A. All three frontends are in scope:** `web/`, `family-app/ui/`, `kiosk-gui/ui/`. Sections 7 and 8 below.
- **B. Keep the cluster dropdown.** Restyle it only. Step 8 changes accordingly: no "Devnet" chip.
- **C. Leave the duplicate dashboard files untouched.**

### Type change caused by A

The family app and kiosk are in Russian, and Familjen Grotesk has no Cyrillic. One typeface across all three products matters more than the name story, so the system font becomes **Onest**: an open-source grotesk designed for Russian public digital services, with open counters and generous x-height that suit older readers. JetBrains Mono (which has Cyrillic) stays for addresses. On web it loads through `next/font`. The Tauri app and the kiosk run offline, so the OFL font files have to be bundled in the repo (`family-app/ui/fonts/`, `kiosk-gui/ui/fonts/`).

### Wisteria, adjusted

The two apps need AAA (7:1) contrast for older users, so Wisteria darkens to **`#5B4AA0`** everywhere: 7.2:1 with white text, 6.0:1 on Lichen. Ivy on Lichen is 11.9:1.

## 7. Family app (`family-app/ui/`)

Who uses it: relatives, often older, on a phone, in Russian. The app is already accessibility-first (52 px targets, 18 px base, dark mode, live regions). The redesign keeps all of that and gives it the Trellis identity.

Constraints:
- `src-tauri/` is backend and isn't touched. Neither is any `call(...)` or state logic in `app.js`, `state.js`, `api.js`, `poll.js`, `errors.js`, `input.js`, `format.js`, `dom.js`, or `mock-api.js`.
- `tools/smoke.mjs` finds elements by button text, `input[name=payee]`, `.count`, `#notice`, `banner banner-<kind>` class names, and the *first* `input[type=text]` on a screen. All of these stay exactly as they are.
- The `dom.js` attribute allowlist stays. No inline styles, no `innerHTML`.

Work:
1. `styles.css` rewrite on the same variable names: Lichen/Ivy/Wisteria light theme and an "ivy night" dark theme, Onest via `@font-face`, radius ranks, 52 px targets kept.
2. The home balance card becomes the Ivy vault panel, matching the web dashboard. The daily allowance shows as a bar, drawn from the strings the backend already returns and never parsed as money. The README says the UI never does money math, so if a bar can't be drawn without parsing amounts, it's dropped and the text stays.
3. Header: the Trellis wordmark in place of the bare `<strong>`. Tabs get a Wisteria indicator, and the selected tab is marked by weight and indicator, not colour alone.
4. Lists (payees, guardians, history) become hairline rows with a status dot plus the existing status text.
5. Banners: left-edge colour bar plus tint instead of a 2 px box outline. Same class names.
6. Check: `npm test` and `npm run smoke` (needs Chromium), and screenshots of each `?mock=` state in the browser.

## 8. Kiosk (`kiosk-gui/ui/`, Slint, 800×480 touchscreen)

Who uses it: an older person at home, touch only, no seed phrase. Today it's pure black on white for AAA contrast. The redesign keeps AAA and the 60 px targets, and fixes a real layout bug: `MyWalletScreen` stacks up to 10 beneficiary buttons in one column, which overflows 480 px after about 4.

Constraints:
- `kiosk-gui/src/*.rs` and `tests/navigation.rs` aren't touched.
- Every `in`/`out` property, callback name, the `Screen` enum, the `BeneficiaryItem` struct, and every `accessible-label` and `accessible-role` stays identical. The navigation test drives the UI through those.
- Help ("Помощь") stays in one fixed spot on every screen (WCAG 3.2.6).

Work:
1. `theme.slint`: Lichen background, Ivy text and primary buttons (11.9:1), Wisteria only as a non-text accent, Sap/Rust for results checked to 7:1. Onest bundled and imported.
2. My wallet / Family wallet: an Ivy balance panel on the left (title, balance, wallet switch, help), and on the right beneficiaries in a 2-column grid of 64 px tiles. That fits all 10 within 480 px.
3. Amount select: presets as a 2-column grid of large tiles, with Back in the left panel's fixed spot.
4. Confirm: amount and recipient set large on two lines instead of one string with "→". Send (Ivy) and Cancel (outlined) side by side, full width.
5. Result: a drawn check or cross mark in a circle instead of the ✓/✗ glyph. Same message and OK button, and the 6 s auto-return is kept.
6. Setup: QR on a white tile (keeps scan contrast), the PIN set in JetBrains Mono at 64 px with digit spacing, Skip outlined.
7. Check: `cargo test -p kiosk-gui --test navigation` if it builds on macOS (the `bluer` Bluetooth crate may make it Linux-only). Otherwise, a throwaway Slint preview harness in the scratchpad that renders each screen to PNG for review.
