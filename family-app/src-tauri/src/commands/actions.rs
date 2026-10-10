//! Commands that change something on the chain. Every one of them:
//! 1. refuses while an earlier operation's outcome is unknown,
//! 2. re-reads the vault and re-checks the rule (the screen may be stale),
//! 3. parses amounts exactly,
//! 4. writes the pending record BEFORE sending (core's before-send hook), so
//!    the rule in (1) survives a crash or close during the confirm wait.

use family_app_core::actions as core;
use family_app_core::actions::mint_decimals;
use family_app_core::discovery::{discover_owned_vaults, owned_vault_addresses};
use family_app_core::wallet::request_devnet_airdrop;
use tauri::State;

use super::{
    acquire_busy, blocking, ensure_no_pending, finish, load_view, not_allowed, parse_amount,
    parse_pubkey, pool_already_open, run_money,
};
use crate::config::{usdc_mint, AIRDROP_LAMPORTS};
use crate::dto::{ActionOkDto, ErrorDto, VaultRefDto};
use crate::state::AppState;
use crate::text::{money, short_address};
use crate::view::{permissions, plan_send, SendPlan};

#[tauri::command]
pub async fn deposit(
    state: State<'_, AppState>,
    vault: String,
    amount: String,
) -> Result<ActionOkDto, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        let _busy = acquire_busy(&st)?;
        ensure_no_pending(&st)?;
        let view = load_view(&st, &vault)?;
        let amount = parse_amount(&amount, view.decimals)?;
        let what = format!("Пополнение на {}", money(amount, view.decimals));
        let result = run_money(&st, &what, |hook| {
            core::deposit(
                &st.rpc,
                &st.program_id,
                &st.identity,
                &view.vault,
                amount,
                hook,
            )
        });
        finish(result, "Деньги отправлены на счёт.")
    })
    .await
}

/// Sends to a payee, or, when a pool's daily limit would be exceeded, files
/// a request that a guardian must approve.
#[tauri::command]
pub async fn send_money(
    state: State<'_, AppState>,
    vault: String,
    recipient: String,
    amount: String,
) -> Result<ActionOkDto, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        let _busy = acquire_busy(&st)?;
        ensure_no_pending(&st)?;
        let view = load_view(&st, &vault)?;
        let now = super::now_unix();
        if !permissions(&view, &st.me(), now).send {
            return Err(not_allowed(
                "Отправлять деньги с этого счёта может только его владелец, и нужен получатель, которого уже можно использовать.",
            ));
        }
        let recipient = parse_pubkey(&recipient, "Получатель")?;
        let payee_ok = view
            .payees
            .iter()
            .any(|p| p.address == recipient && p.is_active(now));
        if !payee_ok {
            return Err(ErrorDto::new(
                "invalid",
                "Выберите получателя из списка. Нового получателя можно использовать через 48 часов после добавления.",
            ));
        }
        let amount = parse_amount(&amount, view.decimals)?;
        let what = format!("Отправка {}", money(amount, view.decimals));
        match plan_send(&view, amount) {
            SendPlan::InsufficientFunds => Err(ErrorDto::new(
                "invalid",
                "На счёте не хватает денег для этой суммы.",
            )),
            SendPlan::Direct => {
                let result = run_money(&st, &what, |hook| {
                    core::send(
                        &st.rpc,
                        &st.program_id,
                        &st.identity,
                        &view.vault,
                        &recipient,
                        amount,
                        hook,
                    )
                });
                finish(result, "Деньги отправлены.")
            }
            SendPlan::Request => {
                let result = run_money(&st, &what, |hook| {
                    core::request_withdrawal(
                        &st.rpc,
                        &st.program_id,
                        &st.identity,
                        &view.vault,
                        &recipient,
                        amount,
                        view.next_free_nonce,
                        hook,
                    )
                });
                finish(
                    result,
                    "Это больше суммы, доступной за день. Запрос отправлен: деньги уйдут, когда доверенное лицо его одобрит.",
                )
            }
        }
    })
    .await
}

#[tauri::command]
pub async fn add_payee(
    state: State<'_, AppState>,
    vault: String,
    name: String,
    address: String,
) -> Result<ActionOkDto, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        let _busy = acquire_busy(&st)?;
        ensure_no_pending(&st)?;
        let view = load_view(&st, &vault)?;
        if !permissions(&view, &st.me(), super::now_unix()).add_payee {
            return Err(not_allowed(
                "Добавлять получателей может только владелец счёта.",
            ));
        }
        let name = name.trim().to_string();
        if name.is_empty() || name.len() > 32 {
            return Err(ErrorDto::new(
                "invalid",
                "Имя должно быть непустым и не длиннее 32 байт (около 16 русских букв).",
            ));
        }
        let address = parse_pubkey(&address, "Получатель")?;
        let what = format!("Добавление получателя «{name}»");
        let result = run_money(&st, &what, |hook| {
            core::add_beneficiary(
                &st.rpc,
                &st.program_id,
                &st.identity,
                &view.vault,
                &name,
                &address,
                hook,
            )
        });
        finish(
            result,
            "Получатель добавлен. Пользоваться им можно будет через 48 часов.",
        )
    })
    .await
}

#[tauri::command]
pub async fn add_guardian(
    state: State<'_, AppState>,
    vault: String,
    address: String,
) -> Result<ActionOkDto, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        let _busy = acquire_busy(&st)?;
        ensure_no_pending(&st)?;
        let view = load_view(&st, &vault)?;
        if !permissions(&view, &st.me(), super::now_unix()).add_guardian {
            return Err(not_allowed(
                "Добавить доверенное лицо с этого устройства нельзя: для этого нужна подпись владельца счёта.",
            ));
        }
        let address = parse_pubkey(&address, "Доверенное лицо")?;
        let result = run_money(&st, "Добавление доверенного лица", |hook| {
            core::add_trusted_approver(
                &st.rpc,
                &st.program_id,
                &st.identity,
                &view.vault,
                &address,
                None,
                hook,
            )
        });
        finish(
            result,
            "Доверенное лицо добавлено.",
        )
    })
    .await
}

#[tauri::command]
pub async fn approve_request(
    state: State<'_, AppState>,
    vault: String,
    nonce: u64,
) -> Result<ActionOkDto, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        let _busy = acquire_busy(&st)?;
        ensure_no_pending(&st)?;
        let view = load_view(&st, &vault)?;
        if !permissions(&view, &st.me(), super::now_unix()).approve {
            return Err(not_allowed(
                "Одобрять переводы могут только доверенные лица.",
            ));
        }
        let request = view
            .pending
            .iter()
            .find(|r| r.nonce == nonce)
            .ok_or_else(|| ErrorDto::new("invalid", "Этого запроса уже нет в списке."))?;
        let what = format!(
            "Одобрение перевода на {}",
            money(request.amount, view.decimals)
        );
        let result = run_money(&st, &what, |hook| {
            core::approve_withdrawal(
                &st.rpc,
                &st.program_id,
                &st.identity,
                &view.vault,
                nonce,
                hook,
            )
        });
        finish(result, "Перевод одобрен.")
    })
    .await
}

/// Opens this key's family account (a pool vault) with a daily limit.
#[tauri::command]
pub async fn open_family_account(
    state: State<'_, AppState>,
    daily_limit: String,
) -> Result<VaultRefDto, ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        let _busy = acquire_busy(&st)?;
        ensure_no_pending(&st)?;
        let me = st.me();
        let owned: Vec<_> = discover_owned_vaults(&st.rpc, &st.program_id, &me)?;
        let [pool, _personal] = owned_vault_addresses(&st.program_id, &me);
        if pool_already_open(&pool, &owned) {
            return Err(ErrorDto::new(
                "invalid",
                "Семейный счёт уже открыт. Выберите его в списке счетов вверху экрана.",
            ));
        }
        let mint = usdc_mint();
        let decimals = mint_decimals(&st.rpc, &mint)?;
        let limit = parse_amount(&daily_limit, decimals)?;
        let vault = run_money(
            &st,
            "Открытие семейного счёта",
            |hook| core::open_pool_vault(&st.rpc, &st.program_id, &st.identity, &mint, limit, hook),
        )?;
        Ok(VaultRefDto {
            address: vault.to_string(),
            address_short: short_address(&vault),
            mine: true,
        })
    })
    .await
}

/// Devnet only: asks the public faucet for SOL to pay transaction fees.
#[tauri::command]
pub async fn get_test_sol(state: State<'_, AppState>) -> Result<(), ErrorDto> {
    let st = state.inner().clone();
    blocking(move || {
        request_devnet_airdrop(&st.rpc, &st.me(), AIRDROP_LAMPORTS).map_err(|error| {
            let mut dto = ErrorDto::from(error);
            if dto.kind == "rejected" {
                dto.message = "Бесплатные тестовые SOL сейчас недоступны (у раздачи есть лимит). \
                               Попробуйте позже или пополните адрес вручную."
                    .to_string();
            }
            dto
        })
    })
    .await
}
