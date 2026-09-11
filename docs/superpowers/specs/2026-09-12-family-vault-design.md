# Family Vault — Design Spec

Дата: 2026-09-12
Хакатон: Colosseum "Crypto World's Fair" (build 2026-09-14 → 2026-10-12)
Статус: approved after brainstorming, pre-implementation

## Проблема

Пожилые люди (65+) практически не пользуются crypto-кошельками: сложный UX,
seed-фразы, риск мошенничества. При этом именно они чаще всего становятся
жертвами финансовой эксплуатации через недоверие к новым технологиям и
изоляцию (см. NCRC issue brief, Elliptic elder-crypto-fraud analysis).
Родственники не могут дать пожилому человеку безопасный доступ к деньгам
без риска потери контроля над ситуацией.

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

// Seeds: ["approvers", vault_pubkey] — только для pool
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

## Открытые вопросы / вне скоупа MVP

- Realloc-логика для BeneficiaryList/TrustedApprovers при росте сверх
  fixed capacity — не в MVP.
- Secure element/TPM — roadmap.
- Мультисемейные сценарии (несколько admin на один pool одновременно) —
  не в MVP, один admin создаёт pool.
- Recovery-флоу при потере/краже устройства (переиздание нового
  устройства с доступом к тем же vault-аккаунтам) — не проработано,
  требует отдельного design pass перед реальным использованием.
