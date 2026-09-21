# Kiosk GUI — Blockchain Wiring Design Spec

Дата: 2026-09-21
Статус: approved after brainstorming, pre-implementation
Spec, от которого наследуется:
`docs/superpowers/specs/2026-09-12-family-vault-design.md`,
`docs/superpowers/specs/2026-09-17-kiosk-gui-design.md`
Программа, к которой обращается этот GUI: `family-vault` (уже
реализована и прошла финальный review — см.
`docs/superpowers/plans/2026-09-12-family-vault-core.md`,
`docs/superpowers/plans/2026-09-12-family-vault-approvals.md`)
GUI shell, который этот план дополняет реальной логикой (сейчас работает
на моках): `docs/superpowers/plans/2026-09-17-kiosk-gui-shell.md`,
код в `kiosk-gui/`

## Цель

Заменить моки в `kiosk-gui/src/main.rs` (жёстко закодированные имена,
баланс, `println!`-заглушка вместо отправки) на реальные вызовы к
Solana devnet: чтение `Vault`/`BeneficiaryList`, подпись и отправка
`withdraw`-инструкции локальным keypair'ом.

## Вне скоупа этого plan'а

- **Family web app** (approve withdrawal, настройка лимитов, добавление
  бенефициаров, over-limit `request_withdrawal`-флоу) — отдельное
  веб-приложение для членов семьи, не это устройство и не эта задача.
  Устройство бабушки/дедушки умеет только: показать баланс, показать
  получателей, отправить `withdraw` в пределах того, что программа уже
  разрешает без approval.
- Provisioning-flow (как заполняется config при первой настройке
  устройства) — отдельная задача; этот план предполагает, что config
  файл уже существует к моменту запуска `kiosk-gui`.
- Физические GPIO-кнопки, kiosk OS (Alpine/cage) — отдельные планы.

## Config

Простой TOML-файл, путь передаётся через `--config` CLI-аргумент (без
аргумента — дефолтный путь `/etc/family-vault/kiosk.toml`, с fallback на
`./kiosk.toml` для dev-запуска с рабочей директории `kiosk-gui/`).

```toml
rpc_url = "https://api.devnet.solana.com"
keypair_path = "/etc/family-vault/owner-keypair.json"
program_id = "Fami1yVau1t11111111111111111111111111111"
mint = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"

personal_vault = "..."
personal_beneficiary_list = "..."
pool_vault = "..."
pool_beneficiary_list = "..."
```

`keypair_path` указывает на JSON-файл в формате Solana CLI keypair
(`[u8; 64]` как JSON-массив) — тот же формат, что `solana-keygen new`
создаёт. Соответствует threat model родительского spec's "Signer /
хранение ключа (MVP threat model)": защищает от UI-based атак, не от
физической кражи устройства с root-доступом.

Все шесть pubkey-полей (`program_id`, `mint`, `personal_vault`,
`personal_beneficiary_list`, `pool_vault`, `pool_beneficiary_list`) —
заполняются на этапе provisioning (вне скоупа), читаются один раз при
старте `main()`.

## Архитектура

Новый модуль `kiosk-gui/src/chain.rs`. Blocking
`solana_client::rpc_client::RpcClient`, вызываемый **не** на UI-потоке:
каждый запрос (чтение аккаунтов при старте, отправка withdraw) уходит в
`std::thread::spawn`, результат передаётся обратно в UI через
`slint::invoke_from_event_loop`.

Почему не async/tokio: Slint гоняет свой собственный event loop, не
tokio-рантайм. Поднимать второй асинхронный рантайм внутри ради
нескольких blocking RPC-вызовов — сложность без выгоды для MVP.
`std::thread::spawn` + `invoke_from_event_loop` — тот же паттерн, что
Slint's официальные примеры используют для фоновой работы, и держит
инвариант ETSI EG 203 350 12.4.6 (immediate response ≤0.1s — UI-поток
никогда не блокируется на сеть).

```
kiosk-gui/
├── src/
│   ├── main.rs      -- запуск, on_* callback wiring (уже есть, дополняется)
│   ├── chain.rs      -- новый: RpcClient, чтение/отправка, decimals-конвертация
│   └── ...
```

**Публичный интерфейс `chain.rs`** (сигнатуры, чтобы main.rs не знал
деталей RPC):

```rust
pub struct ChainConfig { /* поля как в TOML выше, распарсено в Pubkey/String */ }

pub struct VaultSnapshot {
    pub balance_text: String,           // "120 USDC", уже с decimals-делением
    pub beneficiaries: Vec<(u8, String)>, // (local id 1..=N, имя из on-chain Beneficiary.name)
}

pub fn load_config(path: &std::path::Path) -> Result<ChainConfig, ConfigError>;

pub fn fetch_vault_snapshot(cfg: &ChainConfig, is_pool: bool) -> Result<VaultSnapshot, ChainError>;

pub fn submit_withdraw(
    cfg: &ChainConfig,
    is_pool: bool,
    beneficiary_id: u8,
    amount_whole_units: u64,
) -> Result<(), ChainError>;

pub enum ChainError {
    NoConnection,       // RPC timeout/connection refused -> "Нет связи"
    ProgramRejected(String), // decoded Anchor error -> "Не получилось"
}
```

`beneficiary_id` — тот же `id: i32` что уже используется в
`BeneficiaryItem` (Slint-модель), просто позиция в списке (1-indexed) —
внутри `submit_withdraw` транслируется в `Beneficiary.address` через
уже загрученный снэпшот (main.rs держит последний `VaultSnapshot` в
`Rc<RefCell<..>>` рядом с моделью, чтобы не перечитывать список на
каждый клик).

## Decimals

USDC (и любой SPL-токен, который используется на devnet для демо) —
жёстко **decimals = 6**, константа в `chain.rs`. Presets в UI остаются
целыми числами ("10 USDC" = 10 внутри), `submit_withdraw` умножает на
`10u64.pow(6)` перед постройкой инструкции. Не читаем decimals с
Mint-аккаунта динамически — лишний RPC-запрос и сложность ради
гибкости, которая не нужна для одного фиксированного demo-токена.

## Recipient token account

`Beneficiary.address` в программе — это wallet pubkey получателя, не
token account. `submit_withdraw` derive'ит ATA:
`spl_associated_token_account::get_associated_token_address(&beneficiary.address, &cfg.mint)`
и передаёт как `recipient_token_account` в инструкцию. Если у
получателя ещё нет ATA под этот mint — транзакция зафейлится
(`AccountNotFound` от токен-программы); это **не** обрабатывается особым
случаем в MVP — падает в общий `ProgramRejected` → "Не получилось.
Попробуй ещё раз или позови [родственника]". ATA получателя должен
существовать до того, как беневициар добавлен в устройство —
предположение, задокументированное здесь как ограничение (создание ATA
получателя — забота family web app / provisioning, не этого GUI).

## Data flow (замена мока)

**Старт `main()`:**
1. Читаем config (CLI arg или дефолтный путь).
2. Фоновый поток: `fetch_vault_snapshot(personal)` +
   `fetch_vault_snapshot(pool)`.
3. `invoke_from_event_loop`: `app.set_balance_text`,
   `app.set_beneficiaries`, `app.set_family_balance_text`,
   `app.set_family_beneficiaries` — те же setter'ы, что моки уже
   вызывают, просто с реальными данными.
4. Если чтение аккаунтов на старте зафейлилось (RPC недоступен на
   запуске) — не даём пустой белый экран: показываем "Нет связи"
   прямо на Экране 1 вместо баланса (детали текста — implementation
   detail плана, не блокирует этот spec).

**`on_send_confirmed` (Экран 3 → Экран 4):**
1. UI сразу показывает "ожидание" state (уже в spec Экрана 3 —
   индикатор без технического текста).
2. Фоновый поток: `submit_withdraw(cfg, is_pool, beneficiary_id, amount)`.
3. `invoke_from_event_loop` по результату:
   - `Ok(())` → `set_result_success(true)`, текст "Готово! Деньги
     отправлены [Имя]", `Screen::Result`.
   - `Err(NoConnection)` → `set_result_success(false)`, текст "Нет
     связи. Попробуй ещё раз позже", `Screen::Result`.
   - `Err(ProgramRejected(_))` → `set_result_success(false)`, текст "Не
     получилось. Попробуй ещё раз или позови [родственника]",
     `Screen::Result`. Сырой текст ошибки — только в лог-файл на
     устройстве (`eprintln!`/`log` crate), никогда не в UI-текст (по
     родительскому spec "Обработка ошибок").

**Family Wallet (`on_family_beneficiary_selected` → сумма → confirm →
send):** та же цепочка, `is_pool = true`. Сумма, превышающая
`daily_limit - spent_today`, приведёт к тому, что программа вернёт
`DailyLimitExceeded` → падает в обычный `ProgramRejected` → "Не
получилось..." (никакого специального UI для over-limit случая в этом
плане — родственник donduct'ит через family web app отдельно).

## Тестирование

- **Unit-тесты (без сети)** в `chain.rs`: конвертация
  `u64 → "N USDC"` и обратно; декодирование `Beneficiary.name`
  (`[u8; 32]` + `name_len`) в `String`; ATA-derivation детерминирован
  (сравнение с известным ожидаемым pubkey для тестового
  owner+mint пары).
- **Ручная проверка на devnet** (задокументирована как явный шаг в
  implementation plan, не автоматизирована): `solana-keygen new` →
  airdrop → деплой программы на devnet (если ещё не задеплоена) →
  `initialize_vault`/`add_beneficiary` через `solana` CLI или
  вспомогательный скрипт → заполнить `kiosk.toml` реальными адресами →
  запустить `kiosk-gui`, дойти до реальной отправки, проверить баланс
  на devnet explorer.
- litesvm тут не годится — GUI не запускается под litesvm-раннером,
  нужен реальный RPC endpoint.

## Sources

- Родительские specs (см. выше) — threat model, closed address book,
  daily limit семантика уже определены там, не переопределяются здесь.
- [Slint docs: Invoking Slint API from Threads](https://slint.dev/docs/rust/slint/) —
  `invoke_from_event_loop` — паттерн для фоновой работы без блокировки UI.

## Открытые вопросы / вне скоупа

- Family web app (approve, лимиты, добавление бенефициаров,
  over-limit request_withdrawal) — отдельный проект, упомянутый
  пользователем, спецификация которого не входит в этот документ.
- Provisioning-flow (как `kiosk.toml` изначально заполняется) —
  отдельная задача.
- Обработка "у получателя ещё нет ATA" отдельным UI-сообщением —
  сейчас падает в общий "Не получилось", можно уточнить позже, если
  окажется частым случаем на демо.
