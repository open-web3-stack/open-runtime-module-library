//! Unit tests for the tokens module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;

#[test]
fn multicurrency_deposit_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert!(!Accounts::<Runtime>::contains_key(CHARLIE, DOT));
		assert_eq!(Tokens::free_balance(DOT, &CHARLIE), 0);
		assert_eq!(Tokens::total_issuance(DOT), 0);
		assert_ok!(Tokens::deposit(DOT, &CHARLIE, 10));
		assert!(Accounts::<Runtime>::contains_key(CHARLIE, DOT));
		assert_eq!(Tokens::free_balance(DOT, &CHARLIE), 10);
		assert_eq!(Tokens::total_issuance(DOT), 10);
	});
}

#[test]
fn multicurrency_withdraw_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert!(Accounts::<Runtime>::contains_key(ALICE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::total_issuance(DOT), 100);
			assert_ok!(Tokens::withdraw(DOT, &ALICE, 99, ExistenceRequirement::AllowDeath));
			assert!(!Accounts::<Runtime>::contains_key(ALICE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::total_issuance(DOT), 1);
		});
}

#[test]
fn multicurrency_transfer_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			assert!(Accounts::<Runtime>::contains_key(ALICE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);
			assert_ok!(<Tokens as MultiCurrency<_>>::transfer(
				DOT,
				&ALICE,
				&BOB,
				99,
				ExistenceRequirement::AllowDeath
			));
			assert!(!Accounts::<Runtime>::contains_key(ALICE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 199);
		});
}

#[test]
fn multicurrency_can_slash_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert!(!Tokens::can_slash(DOT, &ALICE, 101));
			assert!(Tokens::can_slash(DOT, &ALICE, 100));
		});
}

#[test]
fn multicurrency_slash_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			// slashed_amount < amount
			assert_eq!(Tokens::total_issuance(DOT), 100);
			assert_eq!(Tokens::slash(DOT, &ALICE, 50), 0);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(DOT), 50);

			// slashed_amount == amount
			assert_eq!(Tokens::slash(DOT, &ALICE, 51), 1);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::total_issuance(DOT), 0);
		});
}

#[test]
fn multicurrency_extended_update_balance_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::update_balance(DOT, &ALICE, 50));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 150);
			assert_eq!(Tokens::total_issuance(DOT), 250);

			assert_ok!(Tokens::update_balance(DOT, &BOB, -50));
			assert_eq!(Tokens::free_balance(DOT, &BOB), 50);
			assert_eq!(Tokens::total_issuance(DOT), 200);

			assert_noop!(Tokens::update_balance(DOT, &BOB, -60), Error::<Runtime>::BalanceTooLow);
		});
}

#[test]
fn multi_lockable_currency_set_lock_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::set_lock(ID_1, DOT, &ALICE, 10));
			assert_eq!(Tokens::accounts(&ALICE, DOT).frozen, 10);
			assert_eq!(Tokens::accounts(&ALICE, DOT).frozen(), 10);
			assert_eq!(Tokens::locks(ALICE, DOT).len(), 1);
			assert_ok!(Tokens::set_lock(ID_1, DOT, &ALICE, 50));
			assert_eq!(Tokens::accounts(&ALICE, DOT).frozen, 50);
			assert_eq!(Tokens::locks(ALICE, DOT).len(), 1);
			assert_ok!(Tokens::set_lock(ID_2, DOT, &ALICE, 60));
			assert_eq!(Tokens::accounts(&ALICE, DOT).frozen, 60);
			assert_eq!(Tokens::locks(ALICE, DOT).len(), 2);
		});
}

#[test]
fn multi_lockable_currency_extend_lock_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::set_lock(ID_1, DOT, &ALICE, 10));
			assert_eq!(Tokens::locks(ALICE, DOT).len(), 1);
			assert_eq!(Tokens::accounts(&ALICE, DOT).frozen, 10);
			assert_ok!(Tokens::extend_lock(ID_1, DOT, &ALICE, 20));
			assert_eq!(Tokens::locks(ALICE, DOT).len(), 1);
			assert_eq!(Tokens::accounts(&ALICE, DOT).frozen, 20);
			assert_ok!(Tokens::extend_lock(ID_2, DOT, &ALICE, 10));
			assert_ok!(Tokens::extend_lock(ID_1, DOT, &ALICE, 20));
			assert_eq!(Tokens::locks(ALICE, DOT).len(), 2);
		});
}

#[test]
fn multi_lockable_currency_remove_lock_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::set_lock(ID_1, DOT, &ALICE, 10));
			assert_ok!(Tokens::set_lock(ID_2, DOT, &ALICE, 20));
			assert_eq!(Tokens::locks(ALICE, DOT).len(), 2);
			assert_ok!(Tokens::remove_lock(ID_2, DOT, &ALICE));
			assert_eq!(Tokens::locks(ALICE, DOT).len(), 1);
		});
}

#[test]
fn multi_reservable_currency_can_reserve_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert!(Tokens::can_reserve(DOT, &ALICE, 0));
			assert!(!Tokens::can_reserve(DOT, &ALICE, 101));
			assert!(Tokens::can_reserve(DOT, &ALICE, 100));
		});
}

#[test]
fn multi_reservable_currency_slash_reserved_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::reserve(DOT, &ALICE, 50));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(DOT), 100);
			assert_eq!(Tokens::slash_reserved(DOT, &ALICE, 0), 0);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(DOT), 100);
			assert_eq!(Tokens::slash_reserved(DOT, &ALICE, 100), 50);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::total_issuance(DOT), 50);
		});
}

#[test]
fn multi_reservable_currency_reserve_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_noop!(Tokens::reserve(DOT, &ALICE, 101), Error::<Runtime>::BalanceTooLow);
			assert_ok!(Tokens::reserve(DOT, &ALICE, 0));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::total_balance(DOT, &ALICE), 100);
			assert_ok!(Tokens::reserve(DOT, &ALICE, 50));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Reserved {
				currency_id: DOT,
				who: ALICE,
				amount: 50,
			}));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::total_balance(DOT, &ALICE), 100);

			assert_ok!(Tokens::reserve(DOT, &ALICE, 50));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::total_balance(DOT, &ALICE), 100);
			// ensure will not trigger Endowed event
			assert!(System::events().iter().all(|record| !matches!(
				record.event,
				RuntimeEvent::Tokens(crate::Event::Endowed {
					currency_id: DOT,
					who: ALICE,
					amount: _
				})
			)));
		});
}

#[test]
fn multi_reservable_currency_unreserve_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::unreserve(DOT, &ALICE, 0), 0);
			assert_eq!(Tokens::unreserve(DOT, &ALICE, 50), 50);
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Unreserved {
				currency_id: DOT,
				who: ALICE,
				amount: 0,
			}));
			assert_ok!(Tokens::reserve(DOT, &ALICE, 30));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Reserved {
				currency_id: DOT,
				who: ALICE,
				amount: 30,
			}));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 70);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 30);
			assert_eq!(Tokens::unreserve(DOT, &ALICE, 15), 0);
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Unreserved {
				currency_id: DOT,
				who: ALICE,
				amount: 15,
			}));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 85);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 15);
			assert_eq!(Tokens::unreserve(DOT, &ALICE, 30), 15);
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Unreserved {
				currency_id: DOT,
				who: ALICE,
				amount: 15,
			}));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 0);
			// ensure will not trigger Endowed event
			assert!(System::events().iter().all(|record| !matches!(
				record.event,
				RuntimeEvent::Tokens(crate::Event::Endowed {
					currency_id: DOT,
					who: ALICE,
					amount: _
				})
			)));
		});
}

#[test]
fn multi_reservable_currency_repatriate_reserved_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 0);
			assert_eq!(
				Tokens::repatriate_reserved(DOT, &ALICE, &ALICE, 0, BalanceStatus::Free),
				Ok(0)
			);
			assert_eq!(
				Tokens::repatriate_reserved(DOT, &ALICE, &ALICE, 50, BalanceStatus::Free),
				Ok(50)
			);
			// Repatriating from and to the same account, fund is `unreserved`.
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Unreserved {
				currency_id: DOT,
				who: ALICE,
				amount: 0,
			}));

			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 0);

			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);
			assert_eq!(Tokens::reserved_balance(DOT, &BOB), 0);
			assert_ok!(Tokens::reserve(DOT, &BOB, 50));
			assert_eq!(Tokens::free_balance(DOT, &BOB), 50);
			assert_eq!(Tokens::reserved_balance(DOT, &BOB), 50);
			assert_eq!(
				Tokens::repatriate_reserved(DOT, &BOB, &BOB, 60, BalanceStatus::Reserved),
				Ok(10)
			);

			assert_eq!(Tokens::free_balance(DOT, &BOB), 50);
			assert_eq!(Tokens::reserved_balance(DOT, &BOB), 50);

			assert_eq!(
				Tokens::repatriate_reserved(DOT, &BOB, &ALICE, 30, BalanceStatus::Reserved),
				Ok(0)
			);
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::ReserveRepatriated {
				currency_id: DOT,
				from: BOB,
				to: ALICE,
				amount: 30,
				status: BalanceStatus::Reserved,
			}));

			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 30);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 50);
			assert_eq!(Tokens::reserved_balance(DOT, &BOB), 20);

			assert_eq!(
				Tokens::repatriate_reserved(DOT, &BOB, &ALICE, 30, BalanceStatus::Free),
				Ok(10)
			);

			// Actual amount repatriated is 20.
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::ReserveRepatriated {
				currency_id: DOT,
				from: BOB,
				to: ALICE,
				amount: 20,
				status: BalanceStatus::Free,
			}));

			assert_eq!(Tokens::free_balance(DOT, &ALICE), 120);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 30);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 50);
			assert_eq!(Tokens::reserved_balance(DOT, &BOB), 0);
		});
}

#[test]
fn slash_draw_reserved_correct() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::reserve(DOT, &ALICE, 50));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(DOT), 100);

			assert_eq!(Tokens::slash(DOT, &ALICE, 80), 0);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 20);
			assert_eq!(Tokens::total_issuance(DOT), 20);

			assert_eq!(Tokens::slash(DOT, &ALICE, 50), 30);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::total_issuance(DOT), 0);
		});
}

#[test]
fn no_op_if_amount_is_zero() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Tokens::ensure_can_withdraw(DOT, &ALICE, 0));
		assert_ok!(Tokens::transfer(Some(ALICE).into(), BOB, DOT, 0));
		assert_ok!(Tokens::transfer(Some(ALICE).into(), ALICE, DOT, 0));
		assert_ok!(Tokens::deposit(DOT, &ALICE, 0));
		assert_ok!(Tokens::withdraw(DOT, &ALICE, 0, ExistenceRequirement::AllowDeath));
		assert_eq!(Tokens::slash(DOT, &ALICE, 0), 0);
		assert_eq!(Tokens::slash(DOT, &ALICE, 1), 1);
		assert_ok!(Tokens::update_balance(DOT, &ALICE, 0));
	});
}

#[test]
fn transfer_all_trait_should_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (ALICE, BTC, 200)])
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::free_balance(BTC, &ALICE), 200);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 0);

			assert_ok!(<Tokens as TransferAll<AccountId>>::transfer_all(&ALICE, &BOB));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::free_balance(BTC, &ALICE), 0);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);
			assert_eq!(Tokens::free_balance(BTC, &BOB), 200);

			assert_ok!(Tokens::reserve(DOT, &BOB, 1));
			assert_ok!(<Tokens as TransferAll<AccountId>>::transfer_all(&BOB, &ALICE));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 99);
			assert_eq!(Tokens::free_balance(BTC, &ALICE), 200);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 0);
			assert_eq!(Tokens::free_balance(BTC, &BOB), 0);
		});
}

#[test]
fn named_multi_reservable_currency_slash_reserved_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::reserve_named(&RID_1, DOT, &ALICE, 50));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(DOT), 100);
			assert_eq!(Tokens::slash_reserved_named(&RID_1, DOT, &ALICE, 0), 0);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(DOT), 100);
			assert_eq!(Tokens::slash_reserved_named(&RID_1, DOT, &ALICE, 100), 50);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 0);
			assert_eq!(Tokens::total_issuance(DOT), 50);
		});
}

#[test]
fn named_multi_reservable_currency_reserve_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_noop!(
				Tokens::reserve_named(&RID_1, DOT, &ALICE, 101),
				Error::<Runtime>::BalanceTooLow
			);
			assert_ok!(Tokens::reserve_named(&RID_1, DOT, &ALICE, 0));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 0);
			assert_eq!(Tokens::total_balance(DOT, &ALICE), 100);
			assert_ok!(Tokens::reserve_named(&RID_1, DOT, &ALICE, 50));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Reserved {
				currency_id: DOT,
				who: ALICE,
				amount: 50,
			}));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 50);
			assert_eq!(Tokens::total_balance(DOT, &ALICE), 100);

			assert_ok!(Tokens::reserve_named(&RID_2, DOT, &ALICE, 50));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Reserved {
				currency_id: DOT,
				who: ALICE,
				amount: 50,
			}));

			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance_named(&RID_2, DOT, &ALICE), 50);
			assert_eq!(Tokens::total_balance(DOT, &ALICE), 100);

			// ensure will not trigger Endowed event
			assert!(System::events().iter().all(|record| !matches!(
				record.event,
				RuntimeEvent::Tokens(crate::Event::Endowed {
					currency_id: DOT,
					who: ALICE,
					amount: _
				})
			)));
		});
}

#[test]
fn named_multi_reservable_currency_unreserve_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 0);
			assert_eq!(Tokens::unreserve_named(&RID_1, DOT, &ALICE, 0), 0);

			assert_ok!(Tokens::reserve_named(&RID_1, DOT, &ALICE, 30));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Reserved {
				currency_id: DOT,
				who: ALICE,
				amount: 30,
			}));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 70);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 30);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 30);

			assert_ok!(Tokens::reserve_named(&RID_2, DOT, &ALICE, 30));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Reserved {
				currency_id: DOT,
				who: ALICE,
				amount: 30,
			}));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 40);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 60);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 30);
			assert_eq!(Tokens::reserved_balance_named(&RID_2, DOT, &ALICE), 30);

			assert_eq!(Tokens::unreserve_named(&RID_1, DOT, &ALICE, 30), 0);
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Unreserved {
				currency_id: DOT,
				who: ALICE,
				amount: 30,
			}));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 70);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 30);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 0);
			assert_eq!(Tokens::reserved_balance_named(&RID_2, DOT, &ALICE), 30);

			assert_eq!(Tokens::unreserve_named(&RID_2, DOT, &ALICE, 30), 0);
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Unreserved {
				currency_id: DOT,
				who: ALICE,
				amount: 30,
			}));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 0);
			assert_eq!(Tokens::reserved_balance_named(&RID_2, DOT, &ALICE), 0);
			// ensure will not trigger Endowed event
			assert!(System::events().iter().all(|record| !matches!(
				record.event,
				RuntimeEvent::Tokens(crate::Event::Endowed {
					currency_id: DOT,
					who: ALICE,
					amount: _
				})
			)));
		});
}

#[test]
fn named_multi_reservable_currency_repatriate_reserved_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 0);
			assert_eq!(
				Tokens::repatriate_reserved_named(&RID_1, DOT, &ALICE, &ALICE, 0, BalanceStatus::Free),
				Ok(0)
			);
			assert_eq!(
				Tokens::repatriate_reserved_named(&RID_1, DOT, &ALICE, &ALICE, 50, BalanceStatus::Free),
				Ok(50)
			);

			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 0);

			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &BOB), 0);
			assert_ok!(Tokens::reserve_named(&RID_1, DOT, &BOB, 50));
			assert_eq!(Tokens::free_balance(DOT, &BOB), 50);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &BOB), 50);
			assert_eq!(
				Tokens::repatriate_reserved_named(&RID_1, DOT, &BOB, &BOB, 60, BalanceStatus::Reserved),
				Ok(10)
			);

			assert_eq!(Tokens::free_balance(DOT, &BOB), 50);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &BOB), 50);

			assert_eq!(
				Tokens::repatriate_reserved_named(&RID_1, DOT, &BOB, &ALICE, 30, BalanceStatus::Reserved),
				Ok(0)
			);
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::ReserveRepatriated {
				currency_id: DOT,
				from: BOB,
				to: ALICE,
				amount: 30,
				status: BalanceStatus::Reserved,
			}));

			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 30);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 30);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 50);
			assert_eq!(Tokens::reserved_balance(DOT, &BOB), 20);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &BOB), 20);

			assert_eq!(
				Tokens::repatriate_reserved_named(&RID_1, DOT, &BOB, &ALICE, 30, BalanceStatus::Free),
				Ok(10)
			);

			// Actual amount repatriated is 20.
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::ReserveRepatriated {
				currency_id: DOT,
				from: BOB,
				to: ALICE,
				amount: 20,
				status: BalanceStatus::Free,
			}));

			assert_eq!(Tokens::free_balance(DOT, &ALICE), 120);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 30);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 30);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 50);
			assert_eq!(Tokens::reserved_balance(DOT, &BOB), 0);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &BOB), 0);
		});
}

#[test]
fn slashed_reserved_named_works() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::reserve_named(&RID_1, DOT, &ALICE, 50));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(DOT), 100);

			assert_eq!(Tokens::slash_reserved_named(&RID_1, DOT, &ALICE, 20), 0);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 30);
			assert_eq!(Tokens::total_issuance(DOT), 80);

			assert_eq!(Tokens::slash_reserved_named(&RID_1, DOT, &ALICE, 40), 10);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 0);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::total_issuance(DOT), 50);
		});
}

#[test]
fn named_multi_reservable_ensure_named_reserved_works() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::reserve_named(&RID_1, DOT, &ALICE, 50));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(DOT), 100);

			assert_ok!(Tokens::ensure_reserved_named(&RID_1, DOT, &ALICE, 20));
			assert_ok!(Tokens::ensure_reserved_named(&RID_1, DOT, &ALICE, 70));

			assert_eq!(Tokens::free_balance(DOT, &ALICE), 30);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 70);
		});
}

#[test]
fn named_multi_reservable_unreserve_all_named() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::reserve_named(&RID_1, DOT, &ALICE, 50));
			assert_ok!(Tokens::reserve_named(&RID_1, DOT, &ALICE, 20));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 30);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 70);

			let value = Tokens::unreserve_all_named(&RID_1, DOT, &ALICE);
			assert!(value == 70);

			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 0);
		});
}

#[test]
fn named_multi_reservable_slash_all_reserved_named() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::reserve_named(&RID_1, DOT, &ALICE, 50));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 50);

			let value = Tokens::slash_all_reserved_named(&RID_1, DOT, &ALICE);
			assert!(value == 0);

			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 0);
		});
}

#[test]
fn named_multi_reservable_repatriate_all_reserved_named_works() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &ALICE), 0);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &BOB), 0);
			assert_ok!(Tokens::reserve_named(&RID_1, DOT, &ALICE, 50));

			assert_ok!(Tokens::repatriate_all_reserved_named(
				&RID_1,
				DOT,
				&ALICE,
				&BOB,
				BalanceStatus::Reserved
			));

			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);
			assert_eq!(Tokens::reserved_balance_named(&RID_1, DOT, &BOB), 50);

			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::ReserveRepatriated {
				currency_id: DOT,
				from: ALICE,
				to: BOB,
				amount: 50,
				status: BalanceStatus::Reserved,
			}));
		});
}

#[test]
fn slash_hook_works() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			let initial_hook_calls = OnSlashHook::<Runtime>::calls();

			// slashing zero tokens is a no-op
			assert_eq!(Tokens::slash(DOT, &ALICE, 0), 0);
			assert_eq!(OnSlashHook::<Runtime>::calls(), initial_hook_calls);

			assert_eq!(Tokens::slash(DOT, &ALICE, 50), 0);
			assert_eq!(OnSlashHook::<Runtime>::calls(), initial_hook_calls + 1);

			// `slash` calls the hook even if no amount was slashed
			assert_eq!(Tokens::slash(DOT, &ALICE, 100), 50);
			assert_eq!(OnSlashHook::<Runtime>::calls(), initial_hook_calls + 2);
		});
}

#[test]
fn slash_hook_works_for_reserved() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			let initial_slash_hook_calls = OnSlashHook::<Runtime>::calls();

			assert_ok!(Tokens::reserve(DOT, &ALICE, 50));
			// slashing zero tokens is a no-op
			assert_eq!(Tokens::slash_reserved(DOT, &ALICE, 0), 0);
			assert_eq!(OnSlashHook::<Runtime>::calls(), initial_slash_hook_calls);

			assert_eq!(Tokens::slash_reserved(DOT, &ALICE, 50), 0);
			assert_eq!(OnSlashHook::<Runtime>::calls(), initial_slash_hook_calls + 1);

			// `slash_reserved` calls the hook even if no amount was slashed
			assert_eq!(Tokens::slash_reserved(DOT, &ALICE, 50), 50);
			assert_eq!(OnSlashHook::<Runtime>::calls(), initial_slash_hook_calls + 2);
		});
}

#[test]
fn slash_hook_works_for_reserved_named() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			let initial_slash_hook_calls = OnSlashHook::<Runtime>::calls();

			assert_ok!(Tokens::reserve_named(&RID_1, DOT, &ALICE, 10));
			// slashing zero tokens is a no-op
			assert_eq!(Tokens::slash_reserved_named(&RID_1, DOT, &ALICE, 0), 0);
			assert_eq!(OnSlashHook::<Runtime>::calls(), initial_slash_hook_calls);

			assert_eq!(Tokens::slash_reserved_named(&RID_1, DOT, &ALICE, 10), 0);
			assert_eq!(OnSlashHook::<Runtime>::calls(), initial_slash_hook_calls + 1);

			// `slash_reserved_named` calls `slash_reserved` under-the-hood with a
			// value to slash based on the account's balance. Because the account's
			// balance is currently zero, `slash_reserved` will be a no-op and
			// the OnSlash hook will not be called.
			assert_eq!(Tokens::slash_reserved_named(&RID_1, DOT, &ALICE, 50), 50);
			// Same value as previously because of the no-op
			assert_eq!(OnSlashHook::<Runtime>::calls(), initial_slash_hook_calls + 1);
		});
}
