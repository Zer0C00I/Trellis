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
   интерфейс родственника, не с устройства бабушки.
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
| On-chain программа | Anchor (зафиксировать конкретную версию: 1.0.0 + solana-cli 3.1.x + rustc 1.89+, либо стабильная 0.32.1 + solana-cli 2.1.21 — не смешивать) | стандарт индустрии, аудированный |
| RPC-клиент | `solana-client` 3.1.8, `solana-sdk` 4.0.1 | официальные крейты |
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

## Открытые вопросы / вне скоупа MVP

- Realloc-логика для BeneficiaryList/TrustedApprovers при росте сверх
  fixed capacity — не в MVP.
- Secure element/TPM — roadmap.
- Мультисемейные сценарии (несколько admin на один pool одновременно) —
  сознательно не поддерживаем: один физический гаджет = один конкретный
  vault-набор для одного пожилого человека, как реальный кошелёк с
  наличными, не мульти-тенантное устройство.
