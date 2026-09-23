# Family Vault — Design Spec

Дата: 2026-09-12
Хакатон: Colosseum "Crypto World's Fair" (build 2026-09-14 → 2026-10-12)
Статус: approved after brainstorming, pre-implementation

## Проблема

Владение крипто резко падает с возрастом: по данным Pew Research (январь
2026), крипто владеют 26% людей 18-29 и 28% людей 30-49, но только **10%**
среди 50+; в разбивке по полу — у мужчин доля падает с 16.2% (25-34) до
**3.2%** (65+), у женщин с 8.7% до **1.8%**. Только **15%** крипто-юзеров
вообще старше 55. Проблема не ограничена крипто: пожилые — крупнейшая
группа non-internet-users в мире (доля digital-exclusion среди 65+/75+ по
странам — от 21% в Дании до 97% в Китае). Основные барьеры, названные
самими пожилыми пользователями: 78% не успевают за изменениями
технологий, 69% считают их слишком сложными, 52% — что слишком трудно
научиться (источник: rapid review по digital exclusion в older adults,
medRxiv/PMC).

Одновременно именно эта группа — частая цель финансовой эксплуатации
через недоверие к новым технологиям и изоляцию (NCRC issue brief,
Elliptic elder-crypto-fraud analysis). Родственники не могут дать
пожилому человеку безопасный доступ к деньгам без риска потери контроля
над ситуацией — либо полностью исключают его из crypto-экономики, либо
подвергают риску скама через неподготовленное самостоятельное
использование.

## Позиционирование

"Семейный финансовый терминал на Solana" — устройство, которое даёт
пожилому человеку полный контроль над собственными деньгами, при этом
физически исключая основные векторы скама (произвольные адреса,
произвольные dApp-подписи, seed-фразу).

## Этические принципы (фиксированы явно, не default)

1. **Её деньги — её контроль.** Никакой daily-лимит не накладывается на
   личные средства пожилого человека. Ограничивается **кому** можно
   отправить, не **сколько**.
2. **Family pool — общие деньги, общие правила.** Daily-лимит и
   approval-механизм применяются только к добровольно внесённым в общий
   пул средствам, не к личным.
3. **Монетизация не с пожилого человека.** Core-функциональность (баланс,
   отправка одобренным получателям, пул) — бесплатна всегда, без paywall,
   независимо от статуса чьей-либо подписки. Premium (аналитика,
   уведомления, удалённое управление лимитами) — платит родственник,
   опционально, никак не блокирует доступ пожилого человека к своим
   деньгам.

## Архитектура

```
┌─────────────────────────────────────────┐
│  Kiosk device (Alpine + cage + Slint)     │
│  - единственное приложение на устройстве  │
│  - не может открыть произвольный dApp/URL │
│  - подписывает локальным зашифрованным    │
│    keypair (MVP threat model — см. ниже)  │
└──────────────────┬────────────────────────┘
                    │ RPC (solana-client)
                    ▼
┌─────────────────────────────────────────┐
│  Solana devnet                            │
│  └─ FamilyVault program (кастом, Anchor)  │
│      ├─ Vault PDA (personal, is_pool=false)│
│      ├─ Vault PDA (family pool, is_pool=true)│
│      ├─ BeneficiaryList (shared per vault) │
│      ├─ TrustedApprovers (только для pool) │
│      └─ PendingWithdrawal (только для pool)│
└─────────────────────────────────────────┘
```

Токен: USDC (devnet mint `4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU`),
чтобы баланс не был волатильным для пожилого пользователя.

### Почему один тип Vault для personal и pool

Изначально рассматривался гибрид: personal balance через нативный SPL
`approve`/`delegate` механизм, pool — через кастомную программу. Отклонено:
native delegate не поддерживает daily-reset (статичное разрешение "до X",
не "до X в день"), и не может ограничивать **адрес** получателя — только
сумму. Оба требования (стабильный ежедневный лимит там, где он нужен;
closed address book для anti-scam) требуют кастомной логики в любом
случае — значит выгоднее один переиспользуемый тип аккаунта, чем два
разных решения с частичным дублированием.

## Accounts

```rust
// Seeds: ["vault", elder_pubkey, "personal" | "pool"]
struct Vault {
    owner: Pubkey,
    vault_token_account: Pubkey,
    daily_limit: u64,      // u64::MAX для personal (без лимита)
    spent_today: u64,
    last_reset_day: i64,   // unix day, lazy reset внутри withdraw()
    is_pool: bool,
    bump: u8,
}

// Seeds: ["beneficiaries", vault_pubkey]
struct BeneficiaryList {
    vault: Pubkey,
    entries: Vec<Beneficiary>,   // fixed capacity 10, без realloc (MVP)
}
struct Beneficiary {
    address: Pubkey,
    name: String,
    added_at: i64,
    active_after: i64,     // added_at + 48h, anti-scam cooldown
}

// Seeds: ["approvers", vault_pubkey]
// Для pool: одобряет withdrawal сверх daily_limit.
// Для personal: одобряет owner-rotation (recovery) — withdrawal-лимита
// на personal нет, approvers там нужны ТОЛЬКО для recovery-сценария.
struct TrustedApprovers {
    vault: Pubkey,
    entries: Vec<Pubkey>,  // fixed capacity 5
}

// Seeds: ["request", vault_pubkey, nonce] — только для pool
struct PendingWithdrawal {
    vault: Pubkey,
    amount: u64,
    to: Pubkey,
    requested_at: i64,
    approved_by: Option<Pubkey>,
    executed: bool,
}

// Seeds: ["rotation", vault_pubkey, nonce] — personal и pool
struct PendingRotation {
    vault: Pubkey,
    new_owner: Pubkey,
    requested_by: Pubkey,
    requested_at: i64,
    executes_at: i64,   // requested_at + 72h
    cancelled: bool,
}
```

Fixed-size vectors (без `realloc`) — осознанный компромисс: платим чуть
больше rent сразу (доли доллара), экономим время разработки/тестирования
за 4-недельный хакатон. Для production — пересмотреть в пользу realloc
или per-entry PDA.

## Instructions

```rust
initialize_vault(is_pool: bool, daily_limit: u64)
  // daily_limit игнорируется и ставится u64::MAX если !is_pool

add_beneficiary(name: String, address: Pubkey)
  // signer: owner (personal) / admin или trusted approver (pool)
  // active_after = now + 48h

add_trusted_approver(address: Pubkey)
  // signer: admin. только для pool-vault

deposit(amount: u64)
  // personal: только owner. pool: кто угодно

withdraw(amount: u64, to: Pubkey)
  // проверка: to ∈ BeneficiaryList и now >= active_after
  // if is_pool:
  //     lazy reset: if last_reset_day != today → spent_today = 0
  //     if spent_today + amount <= daily_limit: transfer, spent_today += amount
  //     else: создать PendingWithdrawal, деньги не двигаются
  // if !is_pool:
  //     transfer без ограничения суммы (её деньги)

approve_withdrawal(request: Pubkey)
  // signer: любой из TrustedApprovers
  // выполняет transfer сразу (approve + execute в одной инструкции)

request_owner_rotation(new_owner: Pubkey)
  // signer: любой из TrustedApprovers (нужны и на personal vault теперь,
  // не только на pool — см. изменение в Accounts)
  // создаёт PendingRotation { new_owner, requested_at, executes_at: now+72h }

execute_owner_rotation(rotation: Pubkey)
  // permissionless (кто угодно может дёрнуть после истечения timelock)
  // проверка: now >= executes_at
  // vault.owner = new_owner

cancel_owner_rotation(rotation: Pubkey)
  // signer: текущий owner (если ключ жив и это ложная тревога/атака)
  // ИЛИ любой другой trusted approver, не согласный с ротацией
```

## Anti-scam модель

Главный вектор мошенничества над пожилыми — убедить самостоятельно
добавить/одобрить незнакомый адрес (см. Elliptic, KoinX whitelist-scam
analysis). Защита строится на ограничении **кому можно отправить**, не
**сколько**:

1. **Closed address book.** Устройство не имеет UI для ввода произвольного
   адреса — только кнопки с именами уже добавленных получателей.
2. **48h cooldown на новых получателей**, добавляются только через
   интерфейс родственника, не с устройства бабушки. Раз бабушка физически
   не может вызвать это действие с kiosk-устройства, cooldown защищает не
   от неё — а от компрометации того же ключа-`owner` с другой стороны:
   родственник-администратор с фишингом/малварью на своей машине, или
   украденный доступ к keypair-файлу. 48h — окно для **других**
   родственников заметить нового получателя (через уведомление) и
   вмешаться до того, как деньги реально смогут уйти на этот адрес.

   **Экстренный override.** Реальный компромисс: если семье нужно
   добавить получателя действительно срочно (новая сиделка, оплата
   которой не может ждать 48ч), обычный путь никак это не решает.
   Решение — не убирать cooldown, а дать обходной путь с более высокой
   ценой атаки: `add_beneficiary_urgent` требует подписи **двух разных**
   доверенных approver'ов (`TrustedApprovers`, тех же что и в
   pool-vault approval flow) одновременно — не одного родственника,
   не owner'а (владелец структурно не может быть approver'ом,
   см. `CannotApproveOwnWithdrawals`). Получатель становится активен
   немедленно. Поднимает цену атаки с "скомпрометировать одну сессию"
   до "скомпрометировать/обмануть двух независимых людей одновременно" —
   тот же принцип, что уже применяется к add_trusted_approver
   (co-sign существующего approver'а) и owner-rotation (любой
   несогласный approver отменяет). Реализовано, покрыто тестами:
   `programs/family-vault/src/instructions/add_beneficiary_urgent.rs`.
3. **Kiosk = single-purpose.** Не браузер, не dApp-клиент — физически не
   может подключиться к произвольному сайту и подписать что угодно
   (закрывает класс "wallet drainer" атак).
4. **Seed-фраза не существует** для пожилого пользователя — не может быть
   выужена фишингом.
5. **Notify-not-block.** Каждая транзакция уведомляет родственника — не
   блокирует, но даёт человеческий контроль поверх автоматики.

## Signer / хранение ключа (MVP threat model)

Keypair хранится зашифрованным локально на устройстве, расшифровывается
при загрузке ключом с ограниченными правами доступа на диске. **Явное
ограничение**: не защищает от физической кражи устройства с root-доступом
(ключ дешифрования тоже на диске). Смягчается anti-scam моделью выше —
украденный ключ позволяет слать только на уже одобренные адреса, что
заметно родственникам через notify.

Roadmap (не в MVP): secure element / TPM-backed key storage.

## Технологический стек

| Слой | Технология | Обоснование |
|---|---|---|
| On-chain программа | Anchor **1.2.0** + Solana CLI **3.1.10** + rustc 1.96 (проверено эмпирически: собран через `anchor init --test-template litesvm`, `anchor build` и litesvm-тест реально проходят end-to-end) | текущий стабильный toolchain на момент разработки |
| Тестирование программы | `litesvm 0.10.0` + гранулярные крейты (`solana-message`, `solana-transaction`, `solana-signer`, `solana-keypair`) вместо monolithic `solana-sdk` | единственная связка версий, которая не ловит splits между поколениями `solana-hash`/`solana-clock` в графе зависимостей на эту дату — см. ledger `family-vault-core` за детали перебора |
| RPC-клиент (для kiosk-приложения) | `solana-client`, `solana-sdk` — версия зафиксируется отдельно при написании клиентского кода (Task GUI), не обязана совпадать с версией внутри программы/тестов | официальные крейты |
| GUI | Slint (`i-slint-backend-winit`, feature `backend-winit-wayland`), лицензия GPLv3 | Rust-native, touch-friendly, GPL покрывает embedded use-case |
| OS/kiosk | Alpine Linux + `cage` (wayland kiosk compositor, пакет в Alpine edge/community) | минимальный, single-app, референс: `InventorsTale/alpinelinux-kiosk` |
| Devnet тестирование | USDC devnet mint + Circle faucet / spl-token-faucet.com | не нужно эмулировать волатильный SOL-баланс |

Helius — опционален (priority fees, webhooks), не блокер; версии SDK
внутри `helius` crate (solana-client/sdk 3.0) может конфликтовать с более
свежими 4.0.1 — проверить при интеграции, при конфликте работать напрямую
через `solana-client` RPC без Helius.

## Монетизация

Core (баланс, send одобренным получателям, пул) — бесплатно навсегда,
никогда не блокируется неоплаченной подпиской. Premium подписка для
родственника (аналитика, push-уведомления, удалённое управление лимитами
и получателями) — единственный источник монетизации на MVP-этапе.
B2B (лицензирование домам престарелых) — roadmap, не приоритет для
питча, риск pass-through стоимости на резидента требует отдельной
проработки governance.

## Demand validation

**Вторичное доказательство (уже есть, не требует действий):**
- Вирусные документированные случаи вреда: внук уговорил бабушку вложить
  $50K в крипто, потеряла 60% пенсионных накоплений; подросток обнаружил,
  что мать потеряла ~$30K — проблема не гипотетическая.
- Существующие конкуренты уже видят и обслуживают этот сегмент частично:
  **Tangem** (hardware wallet, no seed phrase, позиционируется под
  seniors), приложение **"Family - Crypto Wallet"** в App Store — рынок
  подтверждён другими игроками, но closed-address-book +
  single-purpose kiosk + family-pool combo не занято никем из них
  (differentiator).
- Рыночный рост: взрослые 60+ — уже **17%** всех крипто-владельцев в
  мире, в США **37%** крипто-юзеров — Gen X или Baby Boomers.
  Сегмент растёт быстрее, чем появляются подходящие инструменты под
  него — отсюда и вред из вирусных историй выше.

**Масштаб денежного вреда (FBI/отраслевые данные):**
- Американцы 60+ потеряли почти **$3 млрд** на крипто-скамах только в
  2024 году (FBI).
- Индивидуальные кейсы: пара потеряла **$1.3 млн** пенсионных
  накоплений скамерам, представившимся сотрудниками FTC; мужчина
  потерял **$340K** сбережений через "друга", встреченного онлайн,
  убедившего вложиться в крипто.
- **Romance/"pig butchering" scams** (фейковые романтические отношения,
  затем убеждение вложить в фиктивную крипто-платформу) — один из
  крупнейших источников финансовых потерь пожилых, свыше **$3.5 млрд**
  зарегистрированных убытков за год по этой отдельной категории.
- Паттерн от caregiver-ресурсов: пожилой человек часто отвергает
  предупреждения родственника, если эмоционально вовлечён (романтический
  скам) или доверяет "другу" — что усиливает аргумент за
  **closed address book** (Секция anti-scam) как структурное, а не
  уговорное решение: не полагаться на то, что пожилой человек поверит
  предупреждению постфактум.
- Reddit-кейс (см. Sources) — вирусный пример именно того сценария,
  который наш продукт должен архитектурно исключить: родственник сам
  инициировал рискованное вложение от имени бабушки без структурных
  ограничений на сумму/адрес.

**Founder-insight (личное наблюдение, не формальное исследование):**
Основатели лично наблюдают более раннюю стадию того же разрыва в своём
окружении — старшее поколение вокруг них не перешло даже на банковские
карты, использует наличные. Не заменяет формальные интервью, но честный
personal founder-market-fit сигнал: проблема не гипотетическая, видна
напрямую, до всякой крипты.

## Recovery-флоу (потеря/кража устройства)

`owner`-ключ каждого vault (personal и pool) может быть заменён через
`request_owner_rotation` → 72h timelock → `execute_owner_rotation`.
Инициирует любой из `TrustedApprovers`. Timelock даёт время заметить и
отменить (`cancel_owner_rotation`) — либо самим владельцем (если ключ
жив и это ложная тревога), либо другим approver'ом, не согласным с
ротацией. Это тот же паттерн, что atomic multisig-recovery у стандартных
кошельков (Squads и т.п.), не изобретение нового примитива — только
адаптация timelock+multisig под наш account-дизайн.

Личный вывод (`owner`) не даёт назначить approvers на personal vault
сам себе бесконтрольно — `add_trusted_approver` на personal vault
должен требовать хотя бы одного уже существующего approver либо
задаваться при `initialize_vault` (иначе одна скомпрометированная
сессия могла бы добавить "своего" approver и затем украсть vault через
rotation). Зафиксировать это правило при имплементации.

## Sources

**Colosseum / hackathon правила и критерии**
- [colosseum.com/hackathon](https://colosseum.com/hackathon) — даты (build 09-14→10-12), eligibility, submission requirements
- [Announcing the Solana Frontier Hackathon](https://blog.colosseum.com/announcing-the-solana-frontier-hackathon/)
- [Colosseum Frontier Hackathon 2026 Winners — Solana Compass](https://solanacompass.com/news/colosseum-announces-26-winners-of-the-solana-frontier-hackathon-the-largest-crypto-hackathon-ever)
- [$2.75M in Investments and Prizes — Incrypted](https://incrypted.com/en/the-colosseum-frontier-hackathon-has-officially-kicked-off/)
- [Solana Grants and Funding](https://solana.org/grants-funding)
- [SuperteamCanada: how-to-win-colosseum-hackathon guide](https://github.com/SuperteamCanada/how-to-win-colosseum-hackathon)

**Demand validation**
- [Reddit grandson/$50K crypto loss story — Yahoo Finance](https://finance.yahoo.com/news/reddit-roasting-guy-lost-grandmas-170041767.html)
- [Teen discovers mom lost ~$30K in crypto — AOL](https://www.aol.com/articles/teen-overhears-mom-lost-around-141001200.html)
- [FBI: Americans 60+ lost ~$3B to crypto scams in 2024 — TipRanks](https://www.tipranks.com/news/crypto-scams-are-quietly-draining-seniors-life-savings-heres-how-to-stop-it)
- [Couple lost $1.3M to fake-FTC crypto scam — Global News](https://globalnews.ca/news/9099844/newmarket-senior-grandparent-scam/amp/)
- [$340K life savings lost via online "friend" crypto scam — MEXC News](https://www.mexc.com/news/452750)
- [Romance/"pig butchering" crypto scams targeting elders — Mass.gov](https://www.mass.gov/news/cryptocurrency-scams-are-targeting-older-adults-what-you-need-to-know)
- [Caregiver pushback patterns in elder scam intervention — HelpGuide.org](https://www.helpguide.org/aging/healthy-aging/elder-scams-and-senior-fraud-abuse)
- [Tangem: Best Crypto Wallet for Seniors 2026](https://tangem.com/en/blog/post/best-crypto-wallet-for-seniors/)
- [Family - Crypto Wallet — App Store](https://apps.apple.com/app/id1606779267)
- [Market validation tactics for pre-seed startups — Thinslices](https://www.thinslices.com/insights/market-validation-tactics-for-pre-seed-and-seed-tech-startups)
- [Customer Validation With No Customers — Pitch Deck Fire](https://pitchdeckfire.com/resources/validation-with-no-customers-perfect-pitch-deck-traction/)

**Проблема / статистика**
- [Pew Research crypto ownership by age (via coinlaw.io)](https://coinlaw.io/crypto-ownership-by-generation-statistics/)
- [Motley Fool 2026 Cryptocurrency Investor Trends Survey](https://www.fool.com/money/research/study-americans-cryptocurrency/)
- [Digital Disparities Among Older Adults — PMC](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC12760905/)
- [Rapid review of digital exclusion interventions in older adults — medRxiv](https://www.medrxiv.org/content/10.1101/2024.03.21.24304670.full.pdf)

**Этика eldercare-tech / монетизация**
- [Aging 4.0? Rethinking the ethical framing of technology-assisted eldercare — PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC8332600/)
- [NCRC: FinTech and Older Adults](https://ncrc.org/issue-brief-fintech-and-older-adults/)
- [Elder Financial Exploitation in the Digital Age — JAAPL](https://jaapl.org/content/51/2/173)
- [Beyond technical access in digital eldercare: ethical lag — BMC Geriatrics](https://link.springer.com/article/10.1186/s12877-026-07768-1)
- [Care coordination in B2B and B2C telemonitoring models — PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC5734380/)

**Anti-scam**
- [Elliptic: crypto scams and elder financial exploitation](https://www.elliptic.co/blog/analysis/crypto-scams-and-elder-financial-exploitation-cracking-down-on-fraud-targeting-the-vulnerable)
- [KoinX: Crypto Whitelist Scams](https://www.koinx.com/blog/crypto-whitelist-scams)

**Технологии / крейты**
- [Anchor releases — solana-foundation/anchor](https://github.com/solana-foundation/anchor/releases)
- [solana-client crates.io](https://crates.io/crates/solana-client)
- [solana-sdk crates.io](https://crates.io/crates/solana-sdk)
- [anchor_spl::token::Approve — docs.rs](https://docs.rs/anchor-spl/latest/anchor_spl/token/struct.Approve.html)
- [spl_token::instruction approve/revoke — docs.rs](https://docs.rs/spl-token/latest/spl_token/instruction/enum.TokenInstruction.html)
- [Helius Rust SDK docs](https://www.helius.dev/docs/agents/rust-sdk)
- [Circle Docs: Transfer USDC on Solana (devnet mint + faucet)](https://developers.circle.com/stablecoins/quickstart-transfer-10-usdc-on-solana)
- [i-slint-backend-winit — crates.io](https://crates.io/crates/i-slint-backend-winit)
- [Slint FAQ (license options)](https://github.com/slint-ui/slint/blob/master/FAQ.md)
- [Slint 1.1 Royalty-Free License blog](https://slint.dev/blog/slint-1.1-released)
- [cage — Alpine Linux package](https://pkgs.alpinelinux.org/package/edge/community/x86/cage)
- [InventorsTale/alpinelinux-kiosk (reference build)](https://github.com/InventorsTale/alpinelinux-kiosk)
- [wlroots multi-gpu / QEMU rendering issue](https://github.com/swaywm/wlroots/issues/1181)

## Открытые вопросы / вне скоупа MVP

- Realloc-логика для BeneficiaryList/TrustedApprovers при росте сверх
  fixed capacity — не в MVP.
- Secure element/TPM — roadmap.
- Мультисемейные сценарии (несколько admin на один pool одновременно) —
  сознательно не поддерживаем: один физический гаджет = один конкретный
  vault-набор для одного пожилого человека, как реальный кошелёк с
  наличными, не мульти-тенантное устройство.
