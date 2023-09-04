//! Unit tests for the tokens module.

#![cfg(test)]

use super::*;
use frame_support::assert_ok;
use mock::*;

const REASON: &() = &();

fn events() -> Vec<RuntimeEvent> {
	let evt = System::events().into_iter().map(|evt| evt.event).collect::<Vec<_>>();
	System::reset_events();
	evt
}

#[test]
fn pallet_multicurrency_deposit_events() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(<Tokens as MultiCurrency<AccountId>>::transfer(DOT, &ALICE, &BOB, 10));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Transfer {
				currency_id: DOT,
				from: ALICE,
				to: BOB,
				amount: 10,
			}));

			assert_ok!(<Tokens as MultiCurrency<AccountId>>::deposit(DOT, &ALICE, 10));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Deposited {
				currency_id: DOT,
				who: ALICE,
				amount: 10,
			}));

			assert_ok!(<Tokens as MultiCurrency<AccountId>>::withdraw(DOT, &ALICE, 10));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Withdrawn {
				currency_id: DOT,
				who: ALICE,
				amount: 10,
			}));

			assert_ok!(<Tokens as MultiReservableCurrency<AccountId>>::reserve(DOT, &ALICE, 50));
			assert_eq!(<Tokens as MultiCurrency<AccountId>>::slash(DOT, &ALICE, 60), 0);
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Slashed {
				currency_id: DOT,
				who: ALICE,
				free_amount: 40,
				reserved_amount: 20,
			}));
		});
}

#[test]
fn pallet_multicurrency_extended_deposit_events() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(<Tokens as MultiCurrencyExtended<AccountId>>::update_balance(
				DOT, &ALICE, 500
			));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Deposited {
				currency_id: DOT,
				who: ALICE,
				amount: 500,
			}));
			assert_ok!(<Tokens as MultiCurrencyExtended<AccountId>>::update_balance(
				DOT, &ALICE, -500
			));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Withdrawn {
				currency_id: DOT,
				who: ALICE,
				amount: 500,
			}));
		});
}

#[test]
fn pallet_multi_lockable_currency_deposit_events() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(<Tokens as MultiLockableCurrency<AccountId>>::set_lock(
				[0u8; 8], DOT, &ALICE, 10
			));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::LockSet {
				lock_id: [0u8; 8],
				currency_id: DOT,
				who: ALICE,
				amount: 10,
			}));

			assert_ok!(<Tokens as MultiLockableCurrency<AccountId>>::remove_lock(
				[0u8; 8], DOT, &ALICE
			));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::LockRemoved {
				lock_id: [0u8; 8],
				currency_id: DOT,
				who: ALICE,
			}));
		});
}

#[test]
fn pallet_multi_reservable_currency_deposit_events() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 1000), (BOB, DOT, 1000)])
		.build()
		.execute_with(|| {
			assert_ok!(<Tokens as MultiReservableCurrency<AccountId>>::reserve(
				DOT, &ALICE, 500
			));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Reserved {
				currency_id: DOT,
				who: ALICE,
				amount: 500,
			}));

			assert_eq!(
				<Tokens as MultiReservableCurrency<AccountId>>::slash_reserved(DOT, &ALICE, 300),
				0
			);
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Slashed {
				currency_id: DOT,
				who: ALICE,
				free_amount: 0,
				reserved_amount: 300,
			}));

			assert_eq!(
				<Tokens as MultiReservableCurrency<AccountId>>::unreserve(DOT, &ALICE, 100),
				0
			);
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Unreserved {
				currency_id: DOT,
				who: ALICE,
				amount: 100,
			}));

			assert_ok!(<Tokens as MultiReservableCurrency<AccountId>>::repatriate_reserved(
				DOT,
				&ALICE,
				&BOB,
				100,
				BalanceStatus::Free
			));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::ReserveRepatriated {
				currency_id: DOT,
				from: ALICE,
				to: BOB,
				amount: 100,
				status: BalanceStatus::Free,
			}));
		});
}

#[test]
fn pallet_fungibles_mutate_deposit_events() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(<Tokens as fungibles::Mutate<AccountId>>::mint_into(DOT, &ALICE, 500));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Deposited {
				currency_id: DOT,
				who: ALICE,
				amount: 500,
			}));
			assert_ok!(<Tokens as fungibles::Mutate<AccountId>>::burn_from(
				DOT,
				&ALICE,
				500,
				Precision::Exact,
				Fortitude::Polite
			));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Withdrawn {
				currency_id: DOT,
				who: ALICE,
				amount: 500,
			}));
		});
}

#[test]
fn pallet_fungibles_transfer_deposit_events() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(<Tokens as fungibles::Mutate<AccountId>>::transfer(
				DOT,
				&ALICE,
				&BOB,
				50,
				Preservation::Protect
			));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Transfer {
				currency_id: DOT,
				from: ALICE,
				to: BOB,
				amount: 50,
			}));
		});
}

#[test]
fn pallet_fungibles_unbalanced_deposit_events() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(<Tokens as MultiReservableCurrency<AccountId>>::reserve(DOT, &ALICE, 50));
			assert_ok!(<Tokens as fungibles::Unbalanced<AccountId>>::write_balance(
				DOT, &ALICE, 500
			));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::BalanceSet {
				currency_id: DOT,
				who: ALICE,
				free: 500,
				reserved: 50,
			}));

			<Tokens as fungibles::Unbalanced<AccountId>>::set_total_issuance(DOT, 1000);
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::TotalIssuanceSet {
				currency_id: DOT,
				amount: 1000,
			}));
		});
}

#[test]
fn pallet_fungibles_mutate_hold_deposit_events() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(<Tokens as fungibles::MutateHold<AccountId>>::hold(
				DOT, REASON, &ALICE, 50
			));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Reserved {
				currency_id: DOT,
				who: ALICE,
				amount: 50,
			}));

			assert_ok!(<Tokens as fungibles::MutateHold<AccountId>>::transfer_on_hold(
				DOT,
				REASON,
				&ALICE,
				&BOB,
				50,
				Precision::Exact,
				Restriction::OnHold,
				Fortitude::Polite
			));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::ReserveRepatriated {
				currency_id: DOT,
				from: ALICE,
				to: BOB,
				amount: 50,
				status: BalanceStatus::Reserved,
			}));
			System::reset_events();
			assert_eq!(
				<Tokens as fungibles::MutateHold<AccountId>>::release(DOT, REASON, &BOB, 50, Precision::Exact),
				Ok(50)
			);
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Unreserved {
				currency_id: DOT,
				who: BOB,
				amount: 50,
			}));
		});
}

#[test]
fn currency_adapter_pallet_currency_deposit_events() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			// Use std::mem::forget to get rid the returned imbalance.
			std::mem::forget(<MockCurrencyAdapter as PalletCurrency<AccountId>>::burn(500));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::TotalIssuanceSet {
				currency_id: DOT,
				amount: 0,
			}));

			std::mem::forget(<MockCurrencyAdapter as PalletCurrency<AccountId>>::issue(200));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::TotalIssuanceSet {
				currency_id: DOT,
				amount: 200,
			}));

			assert_ok!(<MockCurrencyAdapter as PalletCurrency<AccountId>>::transfer(
				&ALICE,
				&BOB,
				50,
				ExistenceRequirement::AllowDeath
			));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Transfer {
				currency_id: DOT,
				from: ALICE,
				to: BOB,
				amount: 50,
			}));

			assert_ok!(<Tokens as MultiReservableCurrency<AccountId>>::reserve(DOT, &BOB, 50));
			std::mem::forget(<MockCurrencyAdapter as PalletCurrency<AccountId>>::slash(&BOB, 110));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Slashed {
				currency_id: DOT,
				who: BOB,
				free_amount: 100,
				reserved_amount: 10,
			}));

			std::mem::forget(<MockCurrencyAdapter as PalletCurrency<AccountId>>::make_free_balance_be(&BOB, 200));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::BalanceSet {
				currency_id: DOT,
				who: BOB,
				free: 200,
				reserved: 40,
			}));
		});
}

#[test]
fn pallet_change_locks_events() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Tokens::do_deposit(DOT, &ALICE, 100, false, false));
		assert_ok!(Tokens::do_deposit(BTC, &ALICE, 100, false, false));
		System::reset_events();

		// Locks: [10/DOT]
		assert_ok!(Tokens::set_lock(ID_1, DOT, &ALICE, 10));
		assert!(events().contains(&RuntimeEvent::Tokens(crate::Event::Locked {
			currency_id: DOT,
			who: ALICE,
			amount: 10
		})));

		// Locks: [15/DOT]
		assert_ok!(Tokens::set_lock(ID_1, DOT, &ALICE, 15));
		assert!(events().contains(&RuntimeEvent::Tokens(crate::Event::Locked {
			currency_id: DOT,
			who: ALICE,
			amount: 5
		})));

		// Locks: [15/DOT, 20/BTC]
		assert_ok!(Tokens::set_lock(ID_1, BTC, &ALICE, 20));
		assert!(events().contains(&RuntimeEvent::Tokens(crate::Event::Locked {
			currency_id: BTC,
			who: ALICE,
			amount: 20
		})));

		// Locks: [15/DOT, 20/BTC, 10/DOT]
		assert_ok!(Tokens::set_lock(ID_2, DOT, &ALICE, 10));
		for event in events() {
			match event {
				RuntimeEvent::Tokens(crate::Event::Locked { .. }) => assert!(false, "unexpected lock event"),
				RuntimeEvent::Tokens(crate::Event::Unlocked { .. }) => assert!(false, "unexpected unlock event"),
				_ => continue,
			}
		}

		// Locks: [15/DOT, 20/BTC, 12/DOT]
		assert_ok!(Tokens::set_lock(ID_2, DOT, &ALICE, 12));
		for event in events() {
			match event {
				RuntimeEvent::Tokens(crate::Event::Locked { .. }) => assert!(false, "unexpected lock event"),
				RuntimeEvent::Tokens(crate::Event::Unlocked { .. }) => assert!(false, "unexpected unlock event"),
				_ => continue,
			}
		}

		// Locks: [15/DOT, 20/BTC, 10/DOT]
		assert_ok!(Tokens::set_lock(ID_2, DOT, &ALICE, 10));
		for event in events() {
			match event {
				RuntimeEvent::Tokens(crate::Event::Locked { .. }) => assert!(false, "unexpected lock event"),
				RuntimeEvent::Tokens(crate::Event::Unlocked { .. }) => assert!(false, "unexpected unlock event"),
				_ => continue,
			}
		}

		// Locks: [15/DOT, 20/BTC, 20/DOT]
		assert_ok!(Tokens::set_lock(ID_2, DOT, &ALICE, 20));
		assert!(events().contains(&RuntimeEvent::Tokens(crate::Event::Locked {
			currency_id: DOT,
			who: ALICE,
			amount: 5
		})));

		// Locks: [15/DOT, 20/BTC, 16/DOT]
		assert_ok!(Tokens::set_lock(ID_2, DOT, &ALICE, 16));
		assert!(events().contains(&RuntimeEvent::Tokens(crate::Event::Unlocked {
			currency_id: DOT,
			who: ALICE,
			amount: 4
		})));

		// Locks: [15/DOT, 12/BTC, 16/DOT]
		assert_ok!(Tokens::set_lock(ID_1, BTC, &ALICE, 12));
		assert!(events().contains(&RuntimeEvent::Tokens(crate::Event::Unlocked {
			currency_id: BTC,
			who: ALICE,
			amount: 8
		})));

		// Locks: [15/DOT, 12/BTC]
		assert_ok!(Tokens::remove_lock(ID_2, DOT, &ALICE));
		assert!(events().contains(&RuntimeEvent::Tokens(crate::Event::Unlocked {
			currency_id: DOT,
			who: ALICE,
			amount: 1
		})));
	});
}

#[test]
fn pallet_multi_lockable_currency_extend_lock_events() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			// lock already exists
			assert_ok!(Tokens::set_lock(ID_1, DOT, &ALICE, 10));
			assert_ok!(Tokens::extend_lock(ID_1, DOT, &ALICE, 20));
			assert!(events().contains(&RuntimeEvent::Tokens(crate::Event::LockSet {
				lock_id: ID_1,
				currency_id: DOT,
				who: ALICE,
				amount: 20,
			})));
			// lock doesn't exist
			assert_ok!(Tokens::extend_lock(ID_2, DOT, &ALICE, 10));
			assert!(events().contains(&RuntimeEvent::Tokens(crate::Event::LockSet {
				lock_id: ID_2,
				currency_id: DOT,
				who: ALICE,
				amount: 10,
			})));
			assert_ok!(Tokens::extend_lock(ID_2, DOT, &ALICE, 20));
			assert!(events().contains(&RuntimeEvent::Tokens(crate::Event::LockSet {
				lock_id: ID_2,
				currency_id: DOT,
				who: ALICE,
				amount: 20,
			})));
		});
}
