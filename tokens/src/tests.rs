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
			assert_eq!(Tokens::free_balance(DOT, &DustReceiver::get()), 0);
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

// *************************************************
// tests for inline impl
// *************************************************

#[test]
fn deposit_consequence_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			Tokens::deposit_consequence(
				&CHARLIE,
				DOT,
				0,
				&AccountData {
					free: 1,
					reserved: 0,
					frozen: 0
				}
			)
			.into_result(),
			Ok(())
		);

		// total issuance overflow
		assert_eq!(
			Tokens::deposit_consequence(
				&CHARLIE,
				DOT,
				Balance::max_value(),
				&AccountData {
					free: 1,
					reserved: 0,
					frozen: 0
				}
			)
			.into_result(),
			Err(ArithmeticError::Overflow.into())
		);

		// total balance overflow
		assert_eq!(
			Tokens::deposit_consequence(
				&CHARLIE,
				DOT,
				1,
				&AccountData {
					free: Balance::max_value(),
					reserved: 0,
					frozen: 0
				}
			)
			.into_result(),
			Err(ArithmeticError::Overflow.into())
		);

		// below ed
		assert_eq!(
			Tokens::deposit_consequence(
				&CHARLIE,
				DOT,
				1,
				&AccountData {
					free: 0,
					reserved: 0,
					frozen: 0
				}
			)
			.into_result(),
			Err(TokenError::BelowMinimum.into())
		);

		assert_eq!(
			Tokens::deposit_consequence(
				&CHARLIE,
				DOT,
				1,
				&AccountData {
					free: 1,
					reserved: 0,
					frozen: 0
				}
			)
			.into_result(),
			Ok(())
		);
	});
}

#[test]
fn withdraw_consequence_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			Tokens::withdraw_consequence(
				&ALICE,
				DOT,
				0,
				&AccountData {
					free: 1,
					reserved: 0,
					frozen: 0
				}
			)
			.into_result(),
			Ok(Zero::zero())
		);

		// total issuance underflow
		assert_ok!(Tokens::update_balance(DOT, &ALICE, 2));
		assert_eq!(Tokens::total_issuance(DOT), 2);
		assert_eq!(
			Tokens::withdraw_consequence(
				&ALICE,
				DOT,
				3,
				&AccountData {
					free: 1,
					reserved: 0,
					frozen: 0
				}
			)
			.into_result(),
			Err(ArithmeticError::Underflow.into())
		);

		// total issuance is not enough
		assert_eq!(
			Tokens::withdraw_consequence(
				&ALICE,
				DOT,
				2,
				&AccountData {
					free: 1,
					reserved: 0,
					frozen: 0
				}
			)
			.into_result(),
			Err(TokenError::NoFunds.into())
		);

		// below ED and cannot dec provider
		assert_ok!(Tokens::update_balance(DOT, &ALICE, 2));
		assert_eq!(System::providers(&ALICE), 1);
		assert_ok!(System::inc_consumers(&ALICE));
		assert!(!System::can_dec_provider(&ALICE));
		assert_eq!(
			Tokens::withdraw_consequence(
				&ALICE,
				DOT,
				1,
				&AccountData {
					free: 2,
					reserved: 0,
					frozen: 0
				}
			)
			.into_result(),
			Err(TokenError::WouldDie.into())
		);

		// below ED and can dec provider
		let _ = System::inc_providers(&ALICE);
		assert!(System::can_dec_provider(&ALICE));
		assert_eq!(
			Tokens::withdraw_consequence(
				&ALICE,
				DOT,
				1,
				&AccountData {
					free: 2,
					reserved: 0,
					frozen: 0
				}
			)
			.into_result(),
			Ok(1)
		);

		// free balance is not enough
		assert_eq!(
			Tokens::withdraw_consequence(
				&ALICE,
				DOT,
				2,
				&AccountData {
					free: 1,
					reserved: 1,
					frozen: 0
				}
			)
			.into_result(),
			Err(TokenError::NoFunds.into())
		);

		// less to frozen balance
		assert_eq!(
			Tokens::withdraw_consequence(
				&ALICE,
				DOT,
				2,
				&AccountData {
					free: 2,
					reserved: 0,
					frozen: 2
				}
			)
			.into_result(),
			Err(TokenError::Frozen.into())
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
fn set_free_balance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		/* Scenarios: ED is not zero, account is not in dust removal whitelist */
		assert!(!Accounts::<Runtime>::contains_key(ALICE, DOT));
		assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
		assert_eq!(Tokens::free_balance(DOT, &DustReceiver::get()), 0);
		assert_eq!(Tokens::total_issuance(DOT), 0);

		// when total is below ED, account will be reaped.
		Tokens::set_free_balance(DOT, &ALICE, 1);
		assert!(!Accounts::<Runtime>::contains_key(ALICE, DOT));
		assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
		assert_eq!(Tokens::free_balance(DOT, &DustReceiver::get()), 1);
		// set_free_balance do not change total issuance.
		assert_eq!(Tokens::total_issuance(DOT), 0);

		Tokens::set_free_balance(DOT, &ALICE, 2);
		assert!(Accounts::<Runtime>::contains_key(ALICE, DOT));
		assert_eq!(Tokens::free_balance(DOT, &ALICE), 2);
		assert_eq!(Tokens::free_balance(DOT, &DustReceiver::get()), 1);

		/* Scenarios: ED is not zero, account is in dust removal whitelist */
		assert!(!Accounts::<Runtime>::contains_key(DAVE, DOT));
		assert_eq!(Tokens::free_balance(DOT, &DAVE), 0);
		assert_eq!(Tokens::free_balance(DOT, &DustReceiver::get()), 1);

		// set zero will not create account
		Tokens::set_free_balance(DOT, &DAVE, 0);
		assert!(!Accounts::<Runtime>::contains_key(DAVE, DOT));

		// when total is below ED, account will not be reaped.
		Tokens::set_free_balance(DOT, &DAVE, 1);
		assert!(Accounts::<Runtime>::contains_key(DAVE, DOT));
		assert_eq!(Tokens::free_balance(DOT, &DAVE), 1);
		assert_eq!(Tokens::free_balance(DOT, &DustReceiver::get()), 1);

		/* Scenarios: ED is zero */
		assert!(!Accounts::<Runtime>::contains_key(ALICE, ETH));
		assert_eq!(Tokens::free_balance(ETH, &ALICE), 0);
		assert_eq!(Tokens::free_balance(ETH, &DustReceiver::get()), 0);

		// set zero will create account
		Tokens::set_free_balance(ETH, &ALICE, 0);
		assert!(Accounts::<Runtime>::contains_key(ALICE, ETH));
		assert_eq!(Tokens::free_balance(ETH, &ALICE), 0);
		assert_eq!(Tokens::free_balance(ETH, &DustReceiver::get()), 0);
	});
}

#[test]
fn set_reserved_balance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		/* Scenarios: ED is not zero, account is not in dust removal whitelist */
		assert!(!Accounts::<Runtime>::contains_key(ALICE, DOT));
		assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 0);
		assert_eq!(Tokens::total_issuance(DOT), 0);

		// when total is below ED, account should be reaped.
		Tokens::set_reserved_balance(DOT, &ALICE, 1);
		// but reap it failed because failed to transfer/withdraw dust removal!!!
		assert!(Accounts::<Runtime>::contains_key(ALICE, DOT));
		assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 1);
		// set_reserved_balance do not change total issuance.
		assert_eq!(Tokens::total_issuance(DOT), 0);

		Tokens::set_reserved_balance(DOT, &ALICE, 2);
		assert!(Accounts::<Runtime>::contains_key(ALICE, DOT));
		assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 2);

		/* Scenarios: ED is not zero, account is in dust removal whitelist */
		assert!(!Accounts::<Runtime>::contains_key(DAVE, DOT));
		assert_eq!(Tokens::free_balance(DOT, &DAVE), 0);

		// set zero will not create account
		Tokens::set_reserved_balance(DOT, &DAVE, 0);
		assert!(!Accounts::<Runtime>::contains_key(DAVE, DOT));

		// when total is below ED, account shouldn't be reaped.
		Tokens::set_reserved_balance(DOT, &DAVE, 1);
		assert!(Accounts::<Runtime>::contains_key(DAVE, DOT));
		assert_eq!(Tokens::reserved_balance(DOT, &DAVE), 1);

		/* Scenarios: ED is zero */
		assert!(!Accounts::<Runtime>::contains_key(ALICE, ETH));
		assert_eq!(Tokens::reserved_balance(ETH, &ALICE), 0);

		// set zero will create account
		Tokens::set_reserved_balance(ETH, &ALICE, 0);
		assert!(Accounts::<Runtime>::contains_key(ALICE, ETH));
		assert_eq!(Tokens::reserved_balance(ETH, &ALICE), 0);
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
			assert_eq!(Tokens::free_balance(DOT, &DustReceiver::get()), 0);

			assert_ok!(Tokens::do_transfer(
				DOT,
				&ALICE,
				&BOB,
				99,
				ExistenceRequirement::AllowDeath
			));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 199);
			assert_eq!(Tokens::free_balance(DOT, &DustReceiver::get()), 1);
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

			// even if dave is in dust removal whitelist, but account drain will still cause
			// account be be reaped.
			assert_noop!(
				Tokens::do_transfer(DOT, &DAVE, &BOB, 100, ExistenceRequirement::KeepAlive),
				Error::<Runtime>::KeepAlive
			);

			// as long as do not transfer all balance, even if the total is below ED, the
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
			assert_eq!(Tokens::free_balance(DOT, &DustReceiver::get()), 0);

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
			assert_eq!(Tokens::free_balance(DOT, &DustReceiver::get()), 1);
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

			// even if dave is in dust removal whitelist, but if withdraw all total of it
			// will still cause account reaped.
			assert_noop!(
				Tokens::do_withdraw(DOT, &DAVE, 1, ExistenceRequirement::KeepAlive, true),
				Error::<Runtime>::KeepAlive
			);
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

// *************************************************
// tests for endowed account and remove account
// *************************************************

#[test]
fn endowed_account_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(System::providers(&ALICE), 0);
		assert!(!Accounts::<Runtime>::contains_key(ALICE, DOT));
		Tokens::set_free_balance(DOT, &ALICE, 100);
		System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Endowed {
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
			Tokens::set_free_balance(DOT, &ALICE, 0);
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
			assert_eq!(Tokens::free_balance(DOT, &DustReceiver::get()), 0);
			Tokens::set_free_balance(DOT, &ALICE, 1);
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::DustLost {
				currency_id: DOT,
				who: ALICE,
				amount: 1,
			}));
			assert_eq!(System::providers(&ALICE), 0);
			assert!(!Accounts::<Runtime>::contains_key(ALICE, DOT));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::free_balance(DOT, &DustReceiver::get()), 1);

			// dave is in dust removal whitelist, will not remove its dust even if its total
			// below ED
			assert!(!Accounts::<Runtime>::contains_key(DAVE, DOT));
			assert_eq!(System::providers(&DAVE), 0);
			assert_eq!(Tokens::free_balance(DOT, &DAVE), 0);
			Tokens::set_free_balance(DOT, &DAVE, 1);
			assert!(Accounts::<Runtime>::contains_key(DAVE, DOT));
			assert_eq!(System::providers(&DAVE), 1);
			assert_eq!(Tokens::free_balance(DOT, &DAVE), 1);
			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Endowed {
				currency_id: DOT,
				who: DAVE,
				amount: 1,
			}));
		});
}

#[test]
fn account_survive_due_to_dust_transfer_failure() {
	ExtBuilder::default().build().execute_with(|| {
		let dust_account = DustReceiver::get();
		Tokens::set_free_balance(DOT, &dust_account, 0);
		assert_eq!(Tokens::free_balance(DOT, &dust_account), 0);
		assert_eq!(Tokens::total_balance(DOT, &ALICE), 0);
		assert_eq!(System::providers(&ALICE), 0);
		assert!(!Accounts::<Runtime>::contains_key(ALICE, DOT));

		Tokens::set_reserved_balance(DOT, &ALICE, 1);
		System::assert_last_event(RuntimeEvent::Tokens(crate::Event::DustLost {
			currency_id: DOT,
			who: ALICE,
			amount: 1,
		}));
		assert_eq!(Tokens::free_balance(DOT, &dust_account), 0);
		assert_eq!(Tokens::total_balance(DOT, &ALICE), 1);
		assert_eq!(System::providers(&ALICE), 1);
		assert!(Accounts::<Runtime>::contains_key(ALICE, DOT));
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
