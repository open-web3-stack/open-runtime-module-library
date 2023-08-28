//! Unit tests for the tokens module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use mock::*;
use sp_runtime::{traits::BadOrigin, TokenError};

// *************************************************
// tests for genesis
// *************************************************

#[test]
fn genesis_issuance_should_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);
			assert_eq!(Tokens::free_balance(DOT, &DustReceiverAccount::get()), 0);
			assert_eq!(Tokens::total_issuance(DOT), 200);
		});
}

// *************************************************
// tests for call
// *************************************************

#[test]
fn transfer_should_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::transfer(Some(ALICE).into(), BOB, DOT, 50));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Transfer {
				currency_id: DOT,
				from: ALICE,
				to: BOB,
				amount: 50,
			}));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 150);
			assert_eq!(Tokens::total_issuance(DOT), 200);

			assert_noop!(
				Tokens::transfer(Some(ALICE).into(), BOB, DOT, 60),
				Error::<Runtime>::BalanceTooLow,
			);
			assert_noop!(
				Tokens::transfer(Some(ALICE).into(), CHARLIE, DOT, 1),
				Error::<Runtime>::ExistentialDeposit,
			);
			assert_ok!(Tokens::transfer(Some(ALICE).into(), CHARLIE, DOT, 2));
			assert_eq!(TrackCreatedAccounts::<Runtime>::accounts(), vec![(CHARLIE, DOT)]);

			// imply AllowDeath
			assert!(Accounts::<Runtime>::contains_key(ALICE, DOT));
			assert_ok!(Tokens::transfer(Some(ALICE).into(), BOB, DOT, 48));
			assert!(!Accounts::<Runtime>::contains_key(ALICE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 198);
			assert_eq!(Tokens::total_issuance(DOT), 200);
		});
}

#[test]
fn transfer_keep_alive_should_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);

			// imply KeepAlive
			assert_noop!(
				Tokens::transfer_keep_alive(Some(ALICE).into(), BOB, DOT, 99),
				Error::<Runtime>::KeepAlive,
			);

			assert_ok!(Tokens::transfer_keep_alive(Some(ALICE).into(), BOB, DOT, 98));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Transfer {
				currency_id: DOT,
				from: ALICE,
				to: BOB,
				amount: 98,
			}));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 2);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 198);
		});
}

#[test]
fn transfer_all_keep_alive_should_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_ok!(Tokens::transfer_all(Some(ALICE).into(), CHARLIE, DOT, true));
			System::assert_has_event(RuntimeEvent::Tokens(crate::Event::Transfer {
				currency_id: DOT,
				from: ALICE,
				to: CHARLIE,
				amount: 98,
			}));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 2);

			assert_ok!(Tokens::set_lock(ID_1, DOT, &BOB, 50));
			assert_eq!(Tokens::accounts(&BOB, DOT).frozen, 50);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);
			assert_ok!(Tokens::transfer_all(Some(BOB).into(), CHARLIE, DOT, true));
			System::assert_has_event(RuntimeEvent::Tokens(crate::Event::Transfer {
				currency_id: DOT,
				from: BOB,
				to: CHARLIE,
				amount: 50,
			}));
		});
}

#[test]
fn transfer_all_allow_death_should_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			assert!(Accounts::<Runtime>::contains_key(ALICE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_ok!(Tokens::transfer_all(Some(ALICE).into(), CHARLIE, DOT, false));
			assert_eq!(TrackCreatedAccounts::<Runtime>::accounts(), vec![(CHARLIE, DOT)]);
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Transfer {
				currency_id: DOT,
				from: ALICE,
				to: CHARLIE,
				amount: 100,
			}));
			assert!(!Accounts::<Runtime>::contains_key(ALICE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(TrackKilledAccounts::<Runtime>::accounts(), vec![(ALICE, DOT)]);

			assert_ok!(Tokens::set_lock(ID_1, DOT, &BOB, 50));
			assert_eq!(Tokens::accounts(&BOB, DOT).frozen, 50);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);
			assert_ok!(Tokens::transfer_all(Some(BOB).into(), CHARLIE, DOT, false));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Transfer {
				currency_id: DOT,
				from: BOB,
				to: CHARLIE,
				amount: 50,
			}));
		});
}

#[test]
fn force_transfer_should_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			assert!(Accounts::<Runtime>::contains_key(ALICE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);
			assert_noop!(
				Tokens::force_transfer(Some(ALICE).into(), ALICE, BOB, DOT, 100),
				BadOrigin
			);

			// imply AllowDeath
			assert_ok!(Tokens::force_transfer(RawOrigin::Root.into(), ALICE, BOB, DOT, 100));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Transfer {
				currency_id: DOT,
				from: ALICE,
				to: BOB,
				amount: 100,
			}));
			assert!(!Accounts::<Runtime>::contains_key(ALICE, DOT));
			assert_eq!(TrackKilledAccounts::<Runtime>::accounts(), vec![(ALICE, DOT)]);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 200);
		});
}

#[test]
fn set_balance_should_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			// bad origin
			assert_noop!(Tokens::set_balance(Some(ALICE).into(), ALICE, DOT, 200, 100), BadOrigin);

			// total balance overflow
			assert_noop!(
				Tokens::set_balance(RawOrigin::Root.into(), ALICE, DOT, Balance::max_value(), 1),
				ArithmeticError::Overflow
			);

			// total issurance overflow
			assert_noop!(
				Tokens::set_balance(RawOrigin::Root.into(), ALICE, DOT, Balance::max_value(), 0),
				ArithmeticError::Overflow
			);

			// total issurance overflow
			assert_noop!(
				Tokens::set_balance(RawOrigin::Root.into(), ALICE, DOT, Balance::max_value(), 0),
				ArithmeticError::Overflow
			);

			assert!(Accounts::<Runtime>::contains_key(ALICE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::total_issuance(DOT), 200);

			assert_ok!(Tokens::set_balance(RawOrigin::Root.into(), ALICE, DOT, 200, 100));
			System::assert_has_event(RuntimeEvent::Tokens(crate::Event::BalanceSet {
				currency_id: DOT,
				who: ALICE,
				free: 200,
				reserved: 100,
			}));
			assert!(Accounts::<Runtime>::contains_key(ALICE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 200);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::total_issuance(DOT), 400);

			assert!(Accounts::<Runtime>::contains_key(BOB, DOT));
			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);
			assert_eq!(Tokens::reserved_balance(DOT, &BOB), 0);

			assert_ok!(Tokens::set_balance(RawOrigin::Root.into(), BOB, DOT, 0, 0));
			System::assert_has_event(RuntimeEvent::Tokens(crate::Event::BalanceSet {
				currency_id: DOT,
				who: BOB,
				free: 0,
				reserved: 0,
			}));
			assert!(!Accounts::<Runtime>::contains_key(BOB, DOT));
			assert_eq!(Tokens::free_balance(DOT, &BOB), 0);
			assert_eq!(Tokens::reserved_balance(DOT, &BOB), 0);
			assert_eq!(Tokens::total_issuance(DOT), 300);

			assert!(!Accounts::<Runtime>::contains_key(CHARLIE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &CHARLIE), 0);
			assert_eq!(Tokens::reserved_balance(DOT, &CHARLIE), 0);

			// below ED,
			assert_ok!(Tokens::set_balance(RawOrigin::Root.into(), CHARLIE, DOT, 1, 0));
			System::assert_has_event(RuntimeEvent::Tokens(crate::Event::BalanceSet {
				currency_id: DOT,
				who: CHARLIE,
				free: 0,
				reserved: 0,
			}));
			assert!(!Accounts::<Runtime>::contains_key(CHARLIE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &CHARLIE), 0);
			assert_eq!(Tokens::reserved_balance(DOT, &CHARLIE), 0);
			assert_eq!(Tokens::total_issuance(DOT), 300);
		});
}

// // *************************************************
// // tests for utils function
// // *************************************************

#[test]
fn wipeout_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			Tokens::wipeout(
				DOT,
				&ALICE,
				&AccountData {
					free: 0,
					reserved: 0,
					frozen: 0
				}
			),
			true
		);
		assert_eq!(
			Tokens::wipeout(
				ETH,
				&ALICE,
				&AccountData {
					free: 0,
					reserved: 0,
					frozen: 0
				}
			),
			false
		);
		assert_eq!(
			Tokens::wipeout(
				DOT,
				&DAVE,
				&AccountData {
					free: 0,
					reserved: 0,
					frozen: 0
				}
			),
			false
		);
		assert_eq!(
			Tokens::wipeout(
				ETH,
				&DAVE,
				&AccountData {
					free: 0,
					reserved: 0,
					frozen: 0
				}
			),
			false
		);

		assert_eq!(
			Tokens::wipeout(
				DOT,
				&ALICE,
				&AccountData {
					free: 0,
					reserved: 1,
					frozen: 0
				}
			),
			false
		);
		assert_eq!(
			Tokens::wipeout(
				ETH,
				&ALICE,
				&AccountData {
					free: 0,
					reserved: 1,
					frozen: 0
				}
			),
			false
		);
		assert_eq!(
			Tokens::wipeout(
				DOT,
				&DAVE,
				&AccountData {
					free: 0,
					reserved: 1,
					frozen: 0
				}
			),
			false
		);
		assert_eq!(
			Tokens::wipeout(
				ETH,
				&DAVE,
				&AccountData {
					free: 0,
					reserved: 1,
					frozen: 0
				}
			),
			false
		);

		assert_eq!(
			Tokens::wipeout(
				DOT,
				&ALICE,
				&AccountData {
					free: 1,
					reserved: 0,
					frozen: 0
				}
			),
			true
		);
		assert_eq!(
			Tokens::wipeout(
				ETH,
				&ALICE,
				&AccountData {
					free: 1,
					reserved: 0,
					frozen: 0
				}
			),
			false
		);
		assert_eq!(
			Tokens::wipeout(
				DOT,
				&DAVE,
				&AccountData {
					free: 1,
					reserved: 0,
					frozen: 0
				}
			),
			false
		);
		assert_eq!(
			Tokens::wipeout(
				ETH,
				&DAVE,
				&AccountData {
					free: 1,
					reserved: 0,
					frozen: 0
				}
			),
			false
		);
	});
}

#[test]
fn try_mutate_account_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100), (EVE, DOT, 100)])
		.build()
		.execute_with(|| {
			// mutate existed account, will not trigger Endowed event
			assert_eq!(System::providers(&ALICE), 1);
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), true);
			assert_eq!(
				Tokens::accounts(&ALICE, DOT),
				AccountData {
					free: 100,
					reserved: 0,
					frozen: 0
				}
			);
			assert_ok!(Tokens::try_mutate_account(
				DOT,
				&ALICE,
				|account, _| -> DispatchResult {
					account.free = 50;
					Ok(())
				}
			));
			assert_eq!(System::providers(&ALICE), 1);
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), true);
			assert_eq!(
				Tokens::accounts(&ALICE, DOT),
				AccountData {
					free: 50,
					reserved: 0,
					frozen: 0
				}
			);
			assert!(System::events().iter().all(|record| !matches!(
				record.event,
				RuntimeEvent::Tokens(crate::Event::Endowed {
					currency_id: DOT,
					who: ALICE,
					amount: _
				})
			)));

			// wipe out account has dust, will trigger DustLost event
			assert_ok!(Tokens::try_mutate_account(
				DOT,
				&ALICE,
				|account, _| -> DispatchResult {
					account.free = 1;
					Ok(())
				}
			));
			assert_eq!(System::providers(&ALICE), 0);
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), false);
			assert_eq!(
				Tokens::accounts(&ALICE, DOT),
				AccountData {
					free: 0,
					reserved: 0,
					frozen: 0
				}
			);
			System::assert_has_event(RuntimeEvent::Tokens(crate::Event::DustLost {
				currency_id: DOT,
				who: ALICE,
				amount: 1,
			}));

			// wipe out zero account, will not trigger DustLost event
			assert_eq!(System::providers(&BOB), 1);
			assert_eq!(Accounts::<Runtime>::contains_key(BOB, DOT), true);
			assert_eq!(
				Tokens::accounts(&BOB, DOT),
				AccountData {
					free: 100,
					reserved: 0,
					frozen: 0
				}
			);
			assert_ok!(Tokens::try_mutate_account(DOT, &BOB, |account, _| -> DispatchResult {
				account.free = 0;
				Ok(())
			}));
			assert_eq!(System::providers(&BOB), 0);
			assert_eq!(Accounts::<Runtime>::contains_key(BOB, DOT), false);
			assert_eq!(
				Tokens::accounts(&BOB, DOT),
				AccountData {
					free: 0,
					reserved: 0,
					frozen: 0
				}
			);
			assert!(System::events().iter().all(|record| !matches!(
				record.event,
				RuntimeEvent::Tokens(crate::Event::DustLost {
					currency_id: DOT,
					who: BOB,
					amount: 0
				})
			)));

			// endow new account, will trigger Endowed event
			assert_eq!(System::providers(&CHARLIE), 0);
			assert_eq!(Accounts::<Runtime>::contains_key(CHARLIE, DOT), false);
			assert_eq!(
				Tokens::accounts(&CHARLIE, DOT),
				AccountData {
					free: 0,
					reserved: 0,
					frozen: 0
				}
			);
			assert_ok!(Tokens::try_mutate_account(
				DOT,
				&CHARLIE,
				|account, _| -> DispatchResult {
					account.free = 50;
					Ok(())
				}
			));
			assert_eq!(System::providers(&CHARLIE), 1);
			assert_eq!(Accounts::<Runtime>::contains_key(CHARLIE, DOT), true);
			assert_eq!(
				Tokens::accounts(&CHARLIE, DOT),
				AccountData {
					free: 50,
					reserved: 0,
					frozen: 0
				}
			);
			System::assert_has_event(RuntimeEvent::Tokens(crate::Event::Endowed {
				currency_id: DOT,
				who: CHARLIE,
				amount: 50,
			}));

			// if the account is in DustRemovalWhitelist, will not wipe out account data if
			// free balance is below ED
			assert_ok!(Tokens::try_mutate_account(DOT, &DAVE, |account, _| -> DispatchResult {
				account.free = 1;
				Ok(())
			}));
			assert_eq!(System::providers(&DAVE), 1);
			assert_eq!(Accounts::<Runtime>::contains_key(DAVE, DOT), true);
			assert_eq!(
				Tokens::accounts(&DAVE, DOT),
				AccountData {
					free: 1,
					reserved: 0,
					frozen: 0
				}
			);
			assert!(System::events().iter().all(|record| !matches!(
				record.event,
				RuntimeEvent::Tokens(crate::Event::DustLost {
					currency_id: DOT,
					who: DAVE,
					amount: _
				})
			)));

			// mutate account reserved but free is zero, will not trigger dust removal
			assert_eq!(System::providers(&EVE), 1);
			assert_eq!(Accounts::<Runtime>::contains_key(EVE, DOT), true);
			assert_eq!(
				Tokens::accounts(&EVE, DOT),
				AccountData {
					free: 100,
					reserved: 0,
					frozen: 0
				}
			);
			assert_ok!(Tokens::try_mutate_account(DOT, &EVE, |account, _| -> DispatchResult {
				account.free = 0;
				account.reserved = 1;
				Ok(())
			}));
			assert_eq!(System::providers(&EVE), 1);
			assert_eq!(Accounts::<Runtime>::contains_key(EVE, DOT), true);
			assert_eq!(
				Tokens::accounts(&EVE, DOT),
				AccountData {
					free: 0,
					reserved: 1,
					frozen: 0
				}
			);
			assert!(System::events().iter().all(|record| !matches!(
				record.event,
				RuntimeEvent::Tokens(crate::Event::DustLost {
					currency_id: DOT,
					who: EVE,
					amount: _
				})
			)));
		});
}

#[test]
fn try_mutate_account_handling_dust_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			// try_mutate_account will not handle dust
			let (_, maybe_dust) = Tokens::try_mutate_account(DOT, &ALICE, |account, _| -> DispatchResult {
				account.free = 1;
				Ok(())
			})
			.unwrap();
			assert_eq!(System::providers(&ALICE), 0);
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), false);
			assert_eq!(
				Tokens::accounts(&ALICE, DOT),
				AccountData {
					free: 0,
					reserved: 0,
					frozen: 0
				}
			);
			System::assert_has_event(RuntimeEvent::Tokens(crate::Event::DustLost {
				currency_id: DOT,
				who: ALICE,
				amount: 1,
			}));
			assert_eq!(maybe_dust, Some(1));
			assert_eq!(
				Tokens::accounts(DustReceiverAccount::get(), DOT),
				AccountData {
					free: 0,
					reserved: 0,
					frozen: 0
				}
			);

			// try_mutate_account_handling_dust will handle dust
			assert_ok!(Tokens::try_mutate_account_handling_dust(
				DOT,
				&BOB,
				|account, _| -> DispatchResult {
					account.free = 1;
					Ok(())
				}
			));
			assert_eq!(System::providers(&BOB), 0);
			assert_eq!(Accounts::<Runtime>::contains_key(BOB, DOT), false);
			assert_eq!(
				Tokens::accounts(&BOB, DOT),
				AccountData {
					free: 0,
					reserved: 0,
					frozen: 0
				}
			);
			System::assert_has_event(RuntimeEvent::Tokens(crate::Event::DustLost {
				currency_id: DOT,
				who: BOB,
				amount: 1,
			}));
			assert_eq!(
				Tokens::accounts(DustReceiverAccount::get(), DOT),
				AccountData {
					free: 1,
					reserved: 0,
					frozen: 0
				}
			);
		});
}

#[test]
fn ensure_can_withdraw_should_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);

			assert_noop!(
				Tokens::ensure_can_withdraw(DOT, &ALICE, 101),
				Error::<Runtime>::BalanceTooLow
			);

			assert_ok!(Tokens::set_lock(ID_1, DOT, &ALICE, 50));
			assert_noop!(
				Tokens::ensure_can_withdraw(DOT, &ALICE, 51),
				Error::<Runtime>::LiquidityRestrictions
			);

			assert_ok!(Tokens::ensure_can_withdraw(DOT, &ALICE, 50));
		});
}

#[test]
fn do_transfer_should_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			// always ok when from == to
			assert_ok!(Tokens::do_transfer(
				DOT,
				&ALICE,
				&ALICE,
				101,
				ExistenceRequirement::KeepAlive
			));

			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);
			assert_eq!(Tokens::free_balance(DOT, &CHARLIE), 0);

			assert_noop!(
				Tokens::do_transfer(DOT, &ALICE, &BOB, 101, ExistenceRequirement::KeepAlive),
				Error::<Runtime>::BalanceTooLow
			);
			assert_noop!(
				Tokens::do_transfer(DOT, &ALICE, &CHARLIE, 1, ExistenceRequirement::KeepAlive),
				Error::<Runtime>::ExistentialDeposit
			);

			assert_ok!(Tokens::do_transfer(
				DOT,
				&ALICE,
				&BOB,
				100,
				ExistenceRequirement::AllowDeath
			));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 200);
		});
}

#[test]
fn do_transfer_dust_removal_when_allow_death() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);
			assert_eq!(Tokens::free_balance(DOT, &DustReceiverAccount::get()), 0);

			assert_ok!(Tokens::do_transfer(
				DOT,
				&ALICE,
				&BOB,
				99,
				ExistenceRequirement::AllowDeath
			));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 199);
			assert_eq!(Tokens::free_balance(DOT, &DustReceiverAccount::get()), 1);
		});
}

#[test]
fn do_transfer_report_keep_alive_error_when_ed_is_not_zero() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (DAVE, DOT, 100)])
		.build()
		.execute_with(|| {
			// total of alice will below ED, account will be reaped.
			assert_noop!(
				Tokens::do_transfer(DOT, &ALICE, &BOB, 99, ExistenceRequirement::KeepAlive),
				Error::<Runtime>::KeepAlive
			);

			// if account is in DustRemovalWhitelist, even if the total is below ED, the
			// account will not be reaped.
			assert_eq!(Tokens::free_balance(DOT, &DAVE), 100);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 0);
			assert!(Accounts::<Runtime>::contains_key(DAVE, DOT));
			assert_ok!(Tokens::do_transfer(
				DOT,
				&DAVE,
				&BOB,
				99,
				ExistenceRequirement::KeepAlive
			));
			assert_eq!(Tokens::free_balance(DOT, &DAVE), 1);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 99);
			assert!(Accounts::<Runtime>::contains_key(DAVE, DOT));
		});
}

#[test]
fn do_transfer_will_not_report_keep_alive_error_when_ed_is_zero() {
	ExtBuilder::default()
		.balances(vec![(ALICE, ETH, 100), (DAVE, ETH, 100)])
		.build()
		.execute_with(|| {
			assert!(Accounts::<Runtime>::contains_key(ALICE, ETH));
			assert_eq!(Tokens::free_balance(ETH, &ALICE), 100);
			assert_eq!(Tokens::free_balance(ETH, &BOB), 0);
			assert_ok!(Tokens::do_transfer(
				ETH,
				&ALICE,
				&BOB,
				99,
				ExistenceRequirement::KeepAlive
			));
			assert!(Accounts::<Runtime>::contains_key(ALICE, ETH));
			assert_eq!(Tokens::free_balance(ETH, &ALICE), 1);
			assert_eq!(Tokens::free_balance(ETH, &BOB), 99);

			// account that total is zero will not be reaped because ED is zero
			assert!(Accounts::<Runtime>::contains_key(DAVE, ETH));
			assert_eq!(Tokens::free_balance(ETH, &DAVE), 100);
			assert_ok!(Tokens::do_transfer(
				ETH,
				&DAVE,
				&BOB,
				100,
				ExistenceRequirement::KeepAlive
			));
			assert!(Accounts::<Runtime>::contains_key(DAVE, ETH));
			assert_eq!(Tokens::free_balance(ETH, &DAVE), 0);
			assert_eq!(Tokens::free_balance(ETH, &BOB), 199);
		});
}

#[test]
fn do_transfer_report_keep_alive_error_due_to_cannot_dec_provider_when_allow_death() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (DAVE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert!(System::can_dec_provider(&ALICE));
			assert_ok!(System::inc_consumers(&ALICE));
			assert!(!System::can_dec_provider(&ALICE));
			assert_noop!(
				Tokens::do_transfer(DOT, &ALICE, &BOB, 99, ExistenceRequirement::AllowDeath),
				Error::<Runtime>::KeepAlive
			);

			assert_ok!(Tokens::deposit(BTC, &ALICE, 100));
			assert!(System::can_dec_provider(&ALICE));
			assert_ok!(Tokens::do_transfer(
				DOT,
				&ALICE,
				&BOB,
				99,
				ExistenceRequirement::AllowDeath
			));
		});
}

#[test]
fn do_transfer_report_existential_deposit_error_when_ed_is_not_zero() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_noop!(
				Tokens::do_transfer(DOT, &ALICE, &BOB, 1, ExistenceRequirement::KeepAlive),
				Error::<Runtime>::ExistentialDeposit
			);

			// if receiver is in dust removal whitelist, will not be reaped.
			assert!(!Accounts::<Runtime>::contains_key(DAVE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &DAVE), 0);
			assert_ok!(Tokens::do_transfer(
				DOT,
				&ALICE,
				&DAVE,
				1,
				ExistenceRequirement::KeepAlive
			));
			assert!(Accounts::<Runtime>::contains_key(DAVE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &DAVE), 1);
		});
}

#[test]
fn do_withdraw_should_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			// always ok if amount is zero
			assert!(!Accounts::<Runtime>::contains_key(BOB, DOT));
			assert_ok!(Tokens::do_withdraw(DOT, &BOB, 0, ExistenceRequirement::KeepAlive, true));
			assert!(!Accounts::<Runtime>::contains_key(BOB, DOT));

			assert_noop!(
				Tokens::do_withdraw(DOT, &ALICE, 101, ExistenceRequirement::KeepAlive, true),
				Error::<Runtime>::BalanceTooLow
			);

			assert_ok!(Tokens::set_lock(ID_1, DOT, &ALICE, 10));
			assert_noop!(
				Tokens::do_withdraw(DOT, &ALICE, 91, ExistenceRequirement::KeepAlive, true),
				Error::<Runtime>::LiquidityRestrictions
			);

			// change issuance
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::total_issuance(DOT), 100);
			assert_ok!(Tokens::do_withdraw(
				DOT,
				&ALICE,
				10,
				ExistenceRequirement::KeepAlive,
				true
			));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 90);
			assert_eq!(Tokens::total_issuance(DOT), 90);

			// do not change issuance
			assert_ok!(Tokens::do_withdraw(
				DOT,
				&ALICE,
				10,
				ExistenceRequirement::KeepAlive,
				false
			));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 80);
			assert_eq!(Tokens::total_issuance(DOT), 90);
		});
}

#[test]
fn do_withdraw_dust_removal_when_allow_death() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::total_issuance(DOT), 100);
			assert!(Accounts::<Runtime>::contains_key(ALICE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::free_balance(DOT, &DustReceiverAccount::get()), 0);

			assert_ok!(Tokens::do_withdraw(
				DOT,
				&ALICE,
				99,
				ExistenceRequirement::AllowDeath,
				true
			));
			assert_eq!(Tokens::total_issuance(DOT), 1);
			assert!(!Accounts::<Runtime>::contains_key(ALICE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::free_balance(DOT, &DustReceiverAccount::get()), 1);
		});
}

#[test]
fn do_withdraw_report_keep_alive_error_when_ed_is_not_zero() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (DAVE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_noop!(
				Tokens::do_withdraw(DOT, &ALICE, 99, ExistenceRequirement::KeepAlive, true),
				Error::<Runtime>::KeepAlive
			);

			// dave is in dust removal whitelist, still can withdraw if remainer is not zero
			// but below ED.
			assert!(Accounts::<Runtime>::contains_key(DAVE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &DAVE), 100);
			assert_eq!(Tokens::total_issuance(DOT), 200);
			assert_ok!(Tokens::do_withdraw(
				DOT,
				&DAVE,
				99,
				ExistenceRequirement::KeepAlive,
				true
			));
			assert!(Accounts::<Runtime>::contains_key(DAVE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &DAVE), 1);
			assert_eq!(Tokens::total_issuance(DOT), 101);
		});
}

#[test]
fn do_withdraw_will_not_report_keep_alive_error_when_ed_is_zero() {
	ExtBuilder::default()
		.balances(vec![(ALICE, ETH, 100), (DAVE, ETH, 100)])
		.build()
		.execute_with(|| {
			assert!(Accounts::<Runtime>::contains_key(ALICE, ETH));
			assert_eq!(Tokens::free_balance(ETH, &ALICE), 100);
			assert_eq!(Tokens::total_issuance(ETH), 200);
			assert_ok!(Tokens::do_withdraw(
				ETH,
				&ALICE,
				100,
				ExistenceRequirement::KeepAlive,
				true
			));
			assert!(Accounts::<Runtime>::contains_key(ALICE, ETH));
			assert_eq!(Tokens::free_balance(ETH, &ALICE), 0);
			assert_eq!(Tokens::total_issuance(ETH), 100);

			assert!(Accounts::<Runtime>::contains_key(DAVE, ETH));
			assert_eq!(Tokens::free_balance(ETH, &DAVE), 100);
			assert_ok!(Tokens::do_withdraw(
				ETH,
				&DAVE,
				100,
				ExistenceRequirement::KeepAlive,
				true
			));
			assert!(Accounts::<Runtime>::contains_key(DAVE, ETH));
			assert_eq!(Tokens::free_balance(ETH, &DAVE), 0);
			assert_eq!(Tokens::total_issuance(ETH), 0);
		});
}

#[test]
fn do_deposit_should_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			// always ok if deposit amount is zero
			assert_ok!(Tokens::do_deposit(DOT, &CHARLIE, 0, true, true));
			assert_ok!(Tokens::do_deposit(DOT, &CHARLIE, 0, false, true));

			assert!(!Accounts::<Runtime>::contains_key(CHARLIE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &CHARLIE), 0);
			assert_eq!(Tokens::total_issuance(DOT), 100);
			assert_ok!(Tokens::do_deposit(DOT, &CHARLIE, 10, false, true));
			assert!(Accounts::<Runtime>::contains_key(CHARLIE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &CHARLIE), 10);
			assert_eq!(Tokens::total_issuance(DOT), 110);

			assert!(Accounts::<Runtime>::contains_key(ALICE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_ok!(Tokens::do_deposit(DOT, &ALICE, 10, true, true));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 110);
			assert_eq!(Tokens::total_issuance(DOT), 120);

			assert_noop!(
				Tokens::do_deposit(DOT, &ALICE, Balance::max_value(), false, true),
				ArithmeticError::Overflow
			);

			// do not change issuance
			assert_ok!(Tokens::do_deposit(DOT, &ALICE, 100, true, false));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 210);
			assert_eq!(Tokens::total_issuance(DOT), 120);
		});
}

#[test]
fn do_deposit_report_dead_account_error() {
	ExtBuilder::default().build().execute_with(|| {
		assert!(!Accounts::<Runtime>::contains_key(CHARLIE, DOT));
		assert_noop!(
			Tokens::do_deposit(DOT, &CHARLIE, 10, true, true),
			Error::<Runtime>::DeadAccount
		);
	});
}

#[test]
fn do_deposit_report_existential_deposit_error() {
	ExtBuilder::default().build().execute_with(|| {
		assert!(!Accounts::<Runtime>::contains_key(CHARLIE, DOT));
		assert_noop!(
			Tokens::do_deposit(DOT, &CHARLIE, 1, false, true),
			Error::<Runtime>::ExistentialDeposit
		);

		assert!(!Accounts::<Runtime>::contains_key(DAVE, DOT));
		assert_eq!(Tokens::free_balance(DOT, &DAVE), 0);
		assert_eq!(Tokens::total_issuance(DOT), 0);
		assert_ok!(Tokens::do_deposit(DOT, &DAVE, 1, false, true));
		assert!(Accounts::<Runtime>::contains_key(DAVE, DOT));
		assert_eq!(Tokens::free_balance(DOT, &DAVE), 1);
		assert_eq!(Tokens::total_issuance(DOT), 1);
	});
}

#[test]
fn update_locks_works() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(
				Tokens::accounts(&ALICE, &DOT),
				AccountData {
					free: 100,
					reserved: 0,
					frozen: 0,
				}
			);
			assert_eq!(Tokens::locks(&ALICE, &DOT), vec![]);
			assert_eq!(System::consumers(&ALICE), 0);

			assert_ok!(Tokens::update_locks(
				DOT,
				&ALICE,
				&vec![BalanceLock { id: ID_1, amount: 30 }]
			));
			System::assert_has_event(RuntimeEvent::Tokens(crate::Event::Locked {
				currency_id: DOT,
				who: ALICE,
				amount: 30,
			}));
			assert_eq!(
				Tokens::accounts(&ALICE, &DOT),
				AccountData {
					free: 100,
					reserved: 0,
					frozen: 30,
				}
			);
			assert_eq!(Tokens::locks(&ALICE, &DOT), vec![BalanceLock { id: ID_1, amount: 30 }]);
			assert_eq!(System::consumers(&ALICE), 1);

			assert_ok!(Tokens::update_locks(
				DOT,
				&ALICE,
				&vec![
					BalanceLock { id: ID_1, amount: 30 },
					BalanceLock { id: ID_2, amount: 35 }
				]
			));
			System::assert_has_event(RuntimeEvent::Tokens(crate::Event::Locked {
				currency_id: DOT,
				who: ALICE,
				amount: 5,
			}));
			assert_eq!(
				Tokens::accounts(&ALICE, &DOT),
				AccountData {
					free: 100,
					reserved: 0,
					frozen: 35,
				}
			);
			assert_eq!(
				Tokens::locks(&ALICE, &DOT),
				vec![
					BalanceLock { id: ID_1, amount: 30 },
					BalanceLock { id: ID_2, amount: 35 }
				]
			);
			assert_eq!(System::consumers(&ALICE), 1);

			assert_noop!(
				Tokens::update_locks(
					DOT,
					&ALICE,
					&vec![
						BalanceLock { id: ID_1, amount: 30 },
						BalanceLock { id: ID_2, amount: 35 },
						BalanceLock { id: ID_3, amount: 40 },
					]
				),
				Error::<Runtime>::MaxLocksExceeded
			);
			assert_eq!(
				Tokens::accounts(&ALICE, &DOT),
				AccountData {
					free: 100,
					reserved: 0,
					frozen: 35,
				}
			);
			assert_eq!(
				Tokens::locks(&ALICE, &DOT),
				vec![
					BalanceLock { id: ID_1, amount: 30 },
					BalanceLock { id: ID_2, amount: 35 }
				]
			);
			assert_eq!(System::consumers(&ALICE), 1);

			assert_ok!(Tokens::update_locks(DOT, &ALICE, &vec![]));
			System::assert_has_event(RuntimeEvent::Tokens(crate::Event::Unlocked {
				currency_id: DOT,
				who: ALICE,
				amount: 35,
			}));
			assert_eq!(
				Tokens::accounts(&ALICE, &DOT),
				AccountData {
					free: 100,
					reserved: 0,
					frozen: 0,
				}
			);
			assert_eq!(Tokens::locks(&ALICE, &DOT), vec![]);
			assert_eq!(System::consumers(&ALICE), 0);
		});
}

#[test]
fn do_transfer_reserved_works() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::reserve(DOT, &ALICE, 80));
			assert_ok!(Tokens::update_locks(
				DOT,
				&ALICE,
				&vec![BalanceLock { id: ID_1, amount: 30 }]
			));
			assert_eq!(
				Tokens::accounts(&ALICE, &DOT),
				AccountData {
					free: 20,
					reserved: 80,
					frozen: 30,
				}
			);

			// case 1:
			// slashed == beneficiary
			// Precision::Exact
			// Fortitude::Polite, The freeze lock applies to the total balance, if discount
			// free balance the remaining is not zero, will locks reserved balance also
			// BalanceStatus::Free
			// amount is 80, but avaliable is 70, will fail
			assert_noop!(
				Tokens::do_transfer_reserved(
					DOT,
					&ALICE,
					&ALICE,
					80,
					Precision::Exact,
					Fortitude::Polite,
					BalanceStatus::Free
				),
				TokenError::FundsUnavailable
			);

			// case 2:
			// slashed == beneficiary
			// Precision::BestEffort
			// Fortitude::Polite, The freeze lock applies to the total balance, if discount
			// free balance the remaining is not zero, will locks reserved balance also
			// BalanceStatus::Free
			// amount is 80, avaliable is 70, actual is 70
			// ALICE will unreserve 70
			assert_eq!(
				Tokens::do_transfer_reserved(
					DOT,
					&ALICE,
					&ALICE,
					80,
					Precision::BestEffort,
					Fortitude::Polite,
					BalanceStatus::Free
				),
				Ok(70)
			);
			System::assert_has_event(RuntimeEvent::Tokens(crate::Event::Unreserved {
				currency_id: DOT,
				who: ALICE,
				amount: 70,
			}));
			assert_eq!(
				Tokens::accounts(&ALICE, &DOT),
				AccountData {
					free: 90,
					reserved: 10,
					frozen: 30,
				}
			);

			// revert to origin state
			assert_ok!(Tokens::update_locks(DOT, &ALICE, &vec![]));
			assert_ok!(Tokens::reserve(DOT, &ALICE, 70));
			assert_ok!(Tokens::update_locks(
				DOT,
				&ALICE,
				&vec![BalanceLock { id: ID_1, amount: 30 }]
			));
			assert_eq!(
				Tokens::accounts(&ALICE, &DOT),
				AccountData {
					free: 20,
					reserved: 80,
					frozen: 30,
				}
			);

			// case 3:
			// slashed == beneficiary
			// Precision::Exact
			// Fortitude::Force
			// BalanceStatus::Free
			// amount is 80, but avaliable is 80
			// ALICE will unreserve 80
			assert_eq!(
				Tokens::do_transfer_reserved(
					DOT,
					&ALICE,
					&ALICE,
					80,
					Precision::Exact,
					Fortitude::Force,
					BalanceStatus::Free
				),
				Ok(80)
			);
			System::assert_has_event(RuntimeEvent::Tokens(crate::Event::Unreserved {
				currency_id: DOT,
				who: ALICE,
				amount: 80,
			}));
			assert_eq!(
				Tokens::accounts(&ALICE, &DOT),
				AccountData {
					free: 100,
					reserved: 0,
					frozen: 30,
				}
			);

			// revert to origin state
			assert_ok!(Tokens::update_locks(DOT, &ALICE, &vec![]));
			assert_ok!(Tokens::reserve(DOT, &ALICE, 80));
			assert_ok!(Tokens::update_locks(
				DOT,
				&ALICE,
				&vec![BalanceLock { id: ID_1, amount: 30 }]
			));
			assert_eq!(
				Tokens::accounts(&ALICE, &DOT),
				AccountData {
					free: 20,
					reserved: 80,
					frozen: 30,
				}
			);

			// case 4:
			// slashed == beneficiary
			// Precision::BestEffort
			// Fortitude::Force
			// BalanceStatus::Free
			// amount is 100, but avaliable is 80
			// ALICE will unreserve 80
			assert_eq!(
				Tokens::do_transfer_reserved(
					DOT,
					&ALICE,
					&ALICE,
					100,
					Precision::BestEffort,
					Fortitude::Force,
					BalanceStatus::Free
				),
				Ok(80)
			);
			System::assert_has_event(RuntimeEvent::Tokens(crate::Event::Unreserved {
				currency_id: DOT,
				who: ALICE,
				amount: 80,
			}));
			assert_eq!(
				Tokens::accounts(&ALICE, &DOT),
				AccountData {
					free: 100,
					reserved: 0,
					frozen: 30,
				}
			);

			// revert to origin state
			assert_ok!(Tokens::update_locks(DOT, &ALICE, &vec![]));
			assert_ok!(Tokens::reserve(DOT, &ALICE, 80));
			assert_ok!(Tokens::update_locks(
				DOT,
				&ALICE,
				&vec![BalanceLock { id: ID_1, amount: 30 }]
			));
			assert_eq!(
				Tokens::accounts(&ALICE, &DOT),
				AccountData {
					free: 20,
					reserved: 80,
					frozen: 30,
				}
			);

			// case 5:
			// slashed == beneficiary
			// Precision::BestEffort
			// Fortitude::Force
			// BalanceStatus::Reserved
			// amount is 100, but avaliable is 80
			// nothing happen for ALICE
			assert_eq!(
				Tokens::do_transfer_reserved(
					DOT,
					&ALICE,
					&ALICE,
					100,
					Precision::BestEffort,
					Fortitude::Force,
					BalanceStatus::Reserved
				),
				Ok(80)
			);
			assert_eq!(
				Tokens::accounts(&ALICE, &DOT),
				AccountData {
					free: 20,
					reserved: 80,
					frozen: 30,
				}
			);

			// case 6:
			// slashed == beneficiary
			// Precision::Exact
			// Fortitude::Force
			// BalanceStatus::Reserved
			// amount is 100, but avaliable is 80
			// throw error
			assert_noop!(
				Tokens::do_transfer_reserved(
					DOT,
					&ALICE,
					&ALICE,
					100,
					Precision::Exact,
					Fortitude::Force,
					BalanceStatus::Reserved
				),
				TokenError::FundsUnavailable
			);

			assert_eq!(
				Tokens::accounts(&BOB, &DOT),
				AccountData {
					free: 0,
					reserved: 0,
					frozen: 0,
				}
			);

			// case 7:
			// slashed != beneficiary
			// Precision::Exact
			// Fortitude::Force
			// BalanceStatus::Reserved
			// amount is 20, avaliable is 80
			// ALICE's reserved balance will transfer 20 to BOB's reserved balance
			assert_eq!(
				Tokens::do_transfer_reserved(
					DOT,
					&ALICE,
					&BOB,
					20,
					Precision::Exact,
					Fortitude::Force,
					BalanceStatus::Reserved
				),
				Ok(20)
			);
			System::assert_has_event(RuntimeEvent::Tokens(crate::Event::ReserveRepatriated {
				currency_id: DOT,
				from: ALICE,
				to: BOB,
				amount: 20,
				status: BalanceStatus::Reserved,
			}));
			assert_eq!(
				Tokens::accounts(&ALICE, &DOT),
				AccountData {
					free: 20,
					reserved: 60,
					frozen: 30,
				}
			);
			assert_eq!(
				Tokens::accounts(&BOB, &DOT),
				AccountData {
					free: 0,
					reserved: 20,
					frozen: 0,
				}
			);

			// case 8:
			// slashed != beneficiary
			// Precision::Exact
			// Fortitude::Force
			// BalanceStatus::Free
			// amount is 20, avaliable is 60
			// ALICE's reserved balance will transfer 20 to BOB's free balance
			assert_eq!(
				Tokens::do_transfer_reserved(
					DOT,
					&ALICE,
					&BOB,
					20,
					Precision::Exact,
					Fortitude::Force,
					BalanceStatus::Free
				),
				Ok(20)
			);
			System::assert_has_event(RuntimeEvent::Tokens(crate::Event::ReserveRepatriated {
				currency_id: DOT,
				from: ALICE,
				to: BOB,
				amount: 20,
				status: BalanceStatus::Free,
			}));
			assert_eq!(
				Tokens::accounts(&ALICE, &DOT),
				AccountData {
					free: 20,
					reserved: 40,
					frozen: 30,
				}
			);
			assert_eq!(
				Tokens::accounts(&BOB, &DOT),
				AccountData {
					free: 20,
					reserved: 20,
					frozen: 0,
				}
			);
		});
}

// *************************************************
// tests for endowed account and remove account
// *************************************************

#[test]
fn endowed_account_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(System::providers(&ALICE), 0);
		assert!(!Accounts::<Runtime>::contains_key(ALICE, DOT));
		assert_ok!(Tokens::set_balance(RawOrigin::Root.into(), ALICE, DOT, 100, 0));
		System::assert_has_event(RuntimeEvent::Tokens(crate::Event::Endowed {
			currency_id: DOT,
			who: ALICE,
			amount: 100,
		}));
		assert_eq!(System::providers(&ALICE), 1);
		assert!(Accounts::<Runtime>::contains_key(ALICE, DOT));
	});
}

#[test]
fn remove_account_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(System::providers(&ALICE), 1);
			assert!(Accounts::<Runtime>::contains_key(ALICE, DOT));
			assert_ok!(Tokens::set_balance(RawOrigin::Root.into(), ALICE, DOT, 0, 0));
			assert_eq!(System::providers(&ALICE), 0);
			assert!(!Accounts::<Runtime>::contains_key(ALICE, DOT));
		});
}

#[test]
fn reap_account_will_dec_providers_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (ALICE, ETH, 100), (ALICE, BTC, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(System::providers(&ALICE), 3);
			assert!(System::account_exists(&ALICE));
			assert!(Accounts::<Runtime>::contains_key(ALICE, DOT));

			assert_ok!(Tokens::do_transfer(
				DOT,
				&ALICE,
				&BOB,
				100,
				ExistenceRequirement::AllowDeath
			));
			assert_eq!(System::providers(&ALICE), 2);
			assert!(System::account_exists(&ALICE));
			assert!(!Accounts::<Runtime>::contains_key(ALICE, DOT));

			// ED of ETH is zero, the account will retain even if the total is zero,
			// will not dec_providers
			assert!(Accounts::<Runtime>::contains_key(ALICE, ETH));
			assert_ok!(Tokens::do_transfer(
				ETH,
				&ALICE,
				&BOB,
				100,
				ExistenceRequirement::AllowDeath
			));
			assert_eq!(System::providers(&ALICE), 2);
			assert!(System::account_exists(&ALICE));
			assert!(Accounts::<Runtime>::contains_key(ALICE, ETH));

			assert!(Accounts::<Runtime>::contains_key(ALICE, BTC));
			assert_ok!(Tokens::do_transfer(
				BTC,
				&ALICE,
				&BOB,
				100,
				ExistenceRequirement::AllowDeath
			));
			assert_eq!(System::providers(&ALICE), 1);
			assert!(System::account_exists(&ALICE));
			assert!(!Accounts::<Runtime>::contains_key(ALICE, BTC));
		});
}

#[test]
fn dust_removal_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(System::providers(&ALICE), 1);
			assert!(Accounts::<Runtime>::contains_key(ALICE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::free_balance(DOT, &DustReceiverAccount::get()), 0);
			assert_eq!(Tokens::total_issuance(DOT), 100);

			// set_balance cannot set free_balance below ED, will set 0
			assert_ok!(Tokens::set_balance(RawOrigin::Root.into(), ALICE, DOT, 1, 0));
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::BalanceSet {
				currency_id: DOT,
				who: ALICE,
				free: 0,
				reserved: 0,
			}));
			assert_eq!(System::providers(&ALICE), 0);
			assert!(!Accounts::<Runtime>::contains_key(ALICE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::free_balance(DOT, &DustReceiverAccount::get()), 0);
			assert_eq!(Tokens::total_issuance(DOT), 0);

			// dave is in dust removal whitelist, will not wipeout
			assert_eq!(System::providers(&DAVE), 0);
			assert!(!Accounts::<Runtime>::contains_key(DAVE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &DAVE), 0);
			assert_ok!(Tokens::set_balance(RawOrigin::Root.into(), DAVE, DOT, 1, 0));
			assert!(Accounts::<Runtime>::contains_key(DAVE, DOT));
			assert_eq!(System::providers(&DAVE), 1);
			assert_eq!(Tokens::free_balance(DOT, &DAVE), 1);
			System::assert_has_event(RuntimeEvent::Tokens(crate::Event::Endowed {
				currency_id: DOT,
				who: DAVE,
				amount: 1,
			}));
		});
}

#[test]
fn account_survive_due_to_dust_transfer_failure() {
	ExtBuilder::default().build().execute_with(|| {
		let dust_account = DustReceiverAccount::get();
		assert_ok!(Tokens::set_balance(
			RawOrigin::Root.into(),
			dust_account.clone(),
			DOT,
			0,
			0
		));
		assert_eq!(Tokens::free_balance(DOT, &dust_account), 0);
		assert_eq!(Tokens::total_balance(DOT, &ALICE), 0);
		assert_eq!(System::providers(&ALICE), 0);
		assert!(!Accounts::<Runtime>::contains_key(ALICE, DOT));

		// set_balance will set zero if the amount will cause wipeout
		assert_ok!(Tokens::set_balance(RawOrigin::Root.into(), ALICE, DOT, 1, 0));
		System::assert_has_event(RuntimeEvent::Tokens(crate::Event::BalanceSet {
			currency_id: DOT,
			who: ALICE,
			free: 0,
			reserved: 0,
		}));
		assert_eq!(Tokens::free_balance(DOT, &dust_account), 0);
		assert_eq!(Tokens::total_balance(DOT, &ALICE), 0);
		assert_eq!(System::providers(&ALICE), 0);
		assert!(!Accounts::<Runtime>::contains_key(ALICE, DOT));
	});
}

#[test]
fn exceeding_max_reserves_should_fail() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			let id_3 = [3u8; 8];
			assert_ok!(Tokens::reserve_named(&RID_1, DOT, &ALICE, 10));
			assert_ok!(Tokens::reserve_named(&RID_2, DOT, &ALICE, 10));
			assert_noop!(
				Tokens::reserve_named(&id_3, DOT, &ALICE, 10),
				Error::<Runtime>::TooManyReserves
			);
		});
}

#[test]
fn lifecycle_callbacks_are_activated() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Tokens::set_balance(RawOrigin::Root.into(), ALICE, DOT, 200, 0));
		assert_eq!(TrackCreatedAccounts::<Runtime>::accounts(), vec![(ALICE, DOT)]);

		assert_ok!(Tokens::set_balance(RawOrigin::Root.into(), ALICE, BTC, 200, 0));
		assert_eq!(
			TrackCreatedAccounts::<Runtime>::accounts(),
			vec![(ALICE, DOT), (ALICE, BTC)]
		);

		assert_ok!(Tokens::transfer_all(Some(ALICE).into(), CHARLIE, BTC, false));
		assert_eq!(
			TrackCreatedAccounts::<Runtime>::accounts(),
			vec![(ALICE, DOT), (ALICE, BTC), (CHARLIE, BTC)]
		);
		assert_eq!(TrackKilledAccounts::<Runtime>::accounts(), vec![(ALICE, BTC)]);
	})
}

// *************************************************
// tests for mutation hooks (OnDeposit, OnTransfer)
// (tests for the OnSlash hook can be found in `./tests_multicurrency.rs`)
// *************************************************

#[test]
fn deposit_hooks_work() {
	ExtBuilder::default().build().execute_with(|| {
		let initial_prehook_calls = PreDeposit::<Runtime>::calls();
		let initial_posthook_calls = PostDeposit::<Runtime>::calls();
		assert_ok!(Tokens::do_deposit(DOT, &CHARLIE, 0, false, true),);
		assert_eq!(PreDeposit::<Runtime>::calls(), initial_prehook_calls);
		assert_eq!(PostDeposit::<Runtime>::calls(), initial_posthook_calls);

		assert_ok!(Tokens::do_deposit(DOT, &CHARLIE, 100, false, true),);
		assert_eq!(PreDeposit::<Runtime>::calls(), initial_prehook_calls + 1);
		assert_eq!(PostDeposit::<Runtime>::calls(), initial_posthook_calls + 1);

		assert_noop!(
			Tokens::do_deposit(DOT, &BOB, 1, false, true),
			Error::<Runtime>::ExistentialDeposit
		);
		// The prehook is called
		assert_eq!(PreDeposit::<Runtime>::calls(), initial_prehook_calls + 2);
		// The posthook is not called
		assert_eq!(PostDeposit::<Runtime>::calls(), initial_posthook_calls + 1);
	});
}

#[test]
fn post_deposit_can_use_new_balance() {
	ExtBuilder::default().build().execute_with(|| {
		let initial_balance = Tokens::free_balance(DOT, &CHARLIE);
		// The following will fail unless Charlie's new balance can be used by the hook,
		// because `initial_balance + 100` is higher than Charlie's initial balance.
		// If this fails, the posthook is called too soon.
		assert_ok!(Tokens::do_deposit(DOT, &CHARLIE, initial_balance + 100, false, true),);
	});
}

#[test]
fn transfer_hooks_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			let initial_prehook_calls = PreTransfer::<Runtime>::calls();
			let initial_posthook_calls = PostTransfer::<Runtime>::calls();
			assert_ok!(Tokens::do_transfer(
				DOT,
				&ALICE,
				&CHARLIE,
				0,
				ExistenceRequirement::AllowDeath
			),);
			assert_eq!(PreTransfer::<Runtime>::calls(), initial_prehook_calls);
			assert_eq!(PostTransfer::<Runtime>::calls(), initial_posthook_calls);

			assert_ok!(Tokens::do_transfer(
				DOT,
				&ALICE,
				&CHARLIE,
				10,
				ExistenceRequirement::AllowDeath
			));
			assert_eq!(PreTransfer::<Runtime>::calls(), initial_prehook_calls + 1);
			assert_eq!(PostTransfer::<Runtime>::calls(), initial_posthook_calls + 1);

			assert_noop!(
				Tokens::do_transfer(DOT, &ALICE, &BOB, 1, ExistenceRequirement::AllowDeath),
				Error::<Runtime>::ExistentialDeposit
			);
			// The prehook is called
			assert_eq!(PreTransfer::<Runtime>::calls(), initial_prehook_calls + 2);
			// The posthook is not called
			assert_eq!(PostTransfer::<Runtime>::calls(), initial_posthook_calls + 1);
		});
}

#[test]
fn post_transfer_can_use_new_balance() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			let initial_balance = Tokens::free_balance(DOT, &CHARLIE);
			// The following will fail unless Charlie's new balance can be used by the hook,
			// because `initial_balance + 100` is higher than Charlie's initial balance.
			// If this fails, the posthook is called too soon.
			assert_ok!(Tokens::do_transfer(
				DOT,
				&ALICE,
				&CHARLIE,
				initial_balance + 100,
				ExistenceRequirement::AllowDeath
			));
		});
}
