//! Turns a core `VaultView` into the screen model, and decides what this
//! key is allowed to do with that vault. Pure: no RPC, no clock (the caller
//! passes `now`), so every rule is unit-tested below.

use family_app_core::vault_view::VaultView;
use solana_pubkey::Pubkey;

use crate::dto::{GuardianDto, PayeeDto, PendingRequestDto, PermissionsDto, VaultDto};
use crate::text::{money, payee_status, short_address};

pub fn is_owner(view: &VaultView, me: &Pubkey) -> bool {
    view.owner == *me
}

pub fn is_guardian(view: &VaultView, me: &Pubkey) -> bool {
    view.guardians.contains(me)
}

/// The rules, in one place:
/// - anyone may deposit;
/// - only the owner sends, adds payees and adds guardians (the program
///   checks the owner's signature);
/// - sending needs at least one payee who is already past the 48-hour wait;
/// - on a personal vault that already has a guardian, adding another needs
///   that guardian to co-sign, which a single device cannot do, so it is
///   not offered;
/// - only a guardian approves, and only when something is waiting.
pub fn permissions(view: &VaultView, me: &Pubkey, now: i64) -> PermissionsDto {
    let owner = is_owner(view, me);
    PermissionsDto {
        deposit: true,
        send: owner && view.payees.iter().any(|p| p.is_active(now)),
        add_payee: owner,
        add_guardian: owner && (view.is_pool || view.guardians.is_empty()),
        approve: is_guardian(view, me) && !view.pending.is_empty(),
    }
}

/// How a send of `amount` base units must be done on this vault.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendPlan {
    /// Within the allowance: send straight away.
    Direct,
    /// Above the daily limit of a pool: file a request a guardian approves.
    Request,
    /// The vault does not hold that much.
    InsufficientFunds,
}

pub fn plan_send(view: &VaultView, amount: u64) -> SendPlan {
    if amount > view.balance {
        SendPlan::InsufficientFunds
    } else if view.is_pool && view.remaining_today().is_some_and(|left| amount > left) {
        SendPlan::Request
    } else {
        SendPlan::Direct
    }
}

pub fn vault_dto(view: &VaultView, me: &Pubkey, now: i64) -> VaultDto {
    let decimals = view.decimals;
    VaultDto {
        address: view.vault.to_string(),
        address_short: short_address(&view.vault),
        title: if view.is_pool {
            "Семейный счёт".to_string()
        } else {
            "Личный счёт".to_string()
        },
        is_pool: view.is_pool,
        is_owner: is_owner(view, me),
        is_guardian: is_guardian(view, me),
        balance: money(view.balance, decimals),
        decimals,
        daily_limit: view.daily_limit.map(|v| money(v, decimals)),
        remaining_today: view.remaining_today().map(|v| money(v, decimals)),
        payees: view
            .payees
            .iter()
            .map(|p| PayeeDto {
                name: p.name.clone(),
                address: p.address.to_string(),
                address_short: short_address(&p.address),
                active: p.is_active(now),
                status: payee_status(p.active_after, now),
            })
            .collect(),
        guardians: view
            .guardians
            .iter()
            .map(|g| GuardianDto {
                address: g.to_string(),
                address_short: short_address(g),
                is_me: g == me,
            })
            .collect(),
        pending: view
            .pending
            .iter()
            .map(|r| PendingRequestDto {
                nonce: r.nonce,
                amount: money(r.amount, decimals),
                to: r.to.to_string(),
                to_short: short_address(&r.to),
                requested_at: r.requested_at,
            })
            .collect(),
        permissions: permissions(view, me, now),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use family_app_core::vault_view::{Payee, PendingRequest};

    const NOW: i64 = 1_000_000;

    fn key(byte: u8) -> Pubkey {
        Pubkey::new_from_array([byte; 32])
    }

    fn payee(byte: u8, active_after: i64) -> Payee {
        Payee {
            name: format!("Получатель {byte}"),
            address: key(byte),
            active_after,
        }
    }

    fn view(owner: u8, is_pool: bool) -> VaultView {
        VaultView {
            vault: key(100),
            owner: key(owner),
            is_pool,
            daily_limit: if is_pool { Some(100_000_000) } else { None },
            spent_today: 30_000_000,
            mint: key(101),
            decimals: 6,
            balance: 250_500_000,
            payees: vec![payee(10, NOW - 1), payee(11, NOW + 3600)],
            guardians: vec![key(2)],
            pending: vec![],
            next_free_nonce: 0,
        }
    }

    fn request() -> PendingRequest {
        PendingRequest {
            nonce: 3,
            address: key(200),
            to: key(10),
            amount: 5_000_000,
            requested_at: NOW - 60,
        }
    }

    #[test]
    fn the_owner_can_send_add_payees_and_guardians_but_not_approve() {
        let me = key(1);
        let p = permissions(&view(1, true), &me, NOW);
        assert!(p.deposit && p.send && p.add_payee && p.add_guardian);
        assert!(!p.approve);
    }

    #[test]
    fn a_guardian_who_is_not_the_owner_can_only_deposit_and_approve() {
        let me = key(2);
        let mut v = view(1, true);
        v.pending.push(request());
        let p = permissions(&v, &me, NOW);
        assert!(p.deposit && p.approve);
        assert!(!p.send && !p.add_payee && !p.add_guardian);
    }

    #[test]
    fn approving_needs_something_to_approve() {
        let p = permissions(&view(1, true), &key(2), NOW);
        assert!(!p.approve);
    }

    #[test]
    fn someone_with_no_role_can_only_deposit() {
        let mut v = view(1, true);
        v.pending.push(request());
        let p = permissions(&v, &key(9), NOW);
        assert!(p.deposit);
        assert!(!p.send && !p.add_payee && !p.add_guardian && !p.approve);
    }

    #[test]
    fn sending_needs_a_payee_who_is_past_the_wait() {
        let me = key(1);
        let mut v = view(1, true);
        v.payees = vec![payee(11, NOW + 3600)];
        assert!(!permissions(&v, &me, NOW).send);
        assert!(permissions(&v, &me, NOW + 3600).send);
        v.payees.clear();
        assert!(!permissions(&v, &me, NOW).send);
    }

    #[test]
    fn a_second_guardian_on_a_personal_vault_needs_a_cosigner_so_it_is_not_offered() {
        let me = key(1);
        // Personal vault that already has a guardian: not offered.
        assert!(!permissions(&view(1, false), &me, NOW).add_guardian);
        // Personal vault with no guardian yet: the owner alone may add the first.
        let mut v = view(1, false);
        v.guardians.clear();
        assert!(permissions(&v, &me, NOW).add_guardian);
        // A pool never needs a cosigner.
        assert!(permissions(&view(1, true), &me, NOW).add_guardian);
    }

    #[test]
    fn a_send_within_the_allowance_goes_straight_through() {
        // 100 limit, 30 spent -> 70 left; balance 250.5
        assert_eq!(plan_send(&view(1, true), 70_000_000), SendPlan::Direct);
        assert_eq!(plan_send(&view(1, true), 1), SendPlan::Direct);
    }

    #[test]
    fn a_send_above_the_daily_limit_of_a_pool_becomes_a_request() {
        assert_eq!(plan_send(&view(1, true), 70_000_001), SendPlan::Request);
        assert_eq!(plan_send(&view(1, true), 200_000_000), SendPlan::Request);
    }

    #[test]
    fn a_personal_vault_never_needs_a_request() {
        assert_eq!(plan_send(&view(1, false), 250_000_000), SendPlan::Direct);
    }

    #[test]
    fn asking_for_more_than_the_balance_is_refused_before_anything_else() {
        assert_eq!(
            plan_send(&view(1, true), 250_500_001),
            SendPlan::InsufficientFunds
        );
        assert_eq!(
            plan_send(&view(1, false), 250_500_001),
            SendPlan::InsufficientFunds
        );
        assert_eq!(plan_send(&view(1, false), 250_500_000), SendPlan::Direct);
    }

    #[test]
    fn the_screen_model_uses_plain_words_and_formatted_money() {
        let mut v = view(1, true);
        v.pending.push(request());
        let dto = vault_dto(&v, &key(1), NOW);
        assert_eq!(dto.title, "Семейный счёт");
        assert_eq!(dto.balance, "250.5 USDC");
        assert_eq!(dto.daily_limit.as_deref(), Some("100 USDC"));
        assert_eq!(dto.remaining_today.as_deref(), Some("70 USDC"));
        assert!(dto.is_owner && !dto.is_guardian);
        assert_eq!(dto.payees.len(), 2);
        assert!(dto.payees[0].active);
        assert_eq!(dto.payees[0].status, "Можно отправлять");
        assert!(!dto.payees[1].active);
        assert_eq!(dto.payees[1].status, "Можно будет через 1 час");
        assert_eq!(dto.pending[0].amount, "5 USDC");
        assert_eq!(dto.pending[0].nonce, 3);
        assert!(dto.guardians[0].address_short.contains('…'));
    }

    #[test]
    fn a_personal_vault_has_no_limit_and_is_titled_personal() {
        let dto = vault_dto(&view(1, false), &key(1), NOW);
        assert_eq!(dto.title, "Личный счёт");
        assert_eq!(dto.daily_limit, None);
        assert_eq!(dto.remaining_today, None);
    }

    #[test]
    fn the_guardian_flag_marks_me_in_the_guardian_list() {
        let dto = vault_dto(&view(1, true), &key(2), NOW);
        assert!(dto.is_guardian);
        assert!(dto.guardians[0].is_me);
    }
}
