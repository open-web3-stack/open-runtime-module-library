//! Unit tests for the tokens module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use mock::{Event, *};
use sp_runtime::{traits::BadOrigin, TokenError};

// *************************************************
// tests for genesis
// *************************************************

#[test]
fn genesis_issuance_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);
			assert_eq!(Tokens::free_balance(DOT, &DustAccount::get()), 2);
			assert_eq!(Tokens::total_issuance(DOT), 202);
		});
}

// *************************************************
// tests for call
// *************************************************

#[test]
fn transfer_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::transfer(Some(ALICE).into(), BOB, DOT, 50));
			System::assert_last_event(Event::Tokens(crate::Event::Transfer(DOT, ALICE, BOB, 50)));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 150);
			assert_eq!(Tokens::total_issuance(DOT), 202);

			assert_noop!(
				Tokens::transfer(Some(ALICE).into(), BOB, DOT, 60),
				Error::<Runtime>::BalanceTooLow,
			);
			assert_noop!(
				Tokens::transfer(Some(ALICE).into(), CHARLIE, DOT, 1),
				Error::<Runtime>::ExistentialDeposit,
			);
			assert_ok!(Tokens::transfer(Some(ALICE).into(), CHARLIE, DOT, 2));

			// imply AllowDeath
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), true);
			assert_ok!(Tokens::transfer(Some(ALICE).into(), BOB, DOT, 48));
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), false);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 198);
			assert_eq!(Tokens::total_issuance(DOT), 202);
		});
}

#[test]
fn transfer_keep_alive_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
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
			System::assert_last_event(Event::Tokens(crate::Event::Transfer(DOT, ALICE, BOB, 98)));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 2);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 198);
		});
}

#[test]
fn transfer_all_keep_alive_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_ok!(Tokens::transfer_all(Some(ALICE).into(), CHARLIE, DOT, true));
			System::assert_has_event(Event::Tokens(crate::Event::Transfer(DOT, ALICE, CHARLIE, 98)));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 2);

			assert_ok!(Tokens::set_lock(ID_1, DOT, &BOB, 50));
			assert_eq!(Tokens::accounts(&BOB, DOT).frozen, 50);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);
			assert_ok!(Tokens::transfer_all(Some(BOB).into(), CHARLIE, DOT, true));
			System::assert_has_event(Event::Tokens(crate::Event::Transfer(DOT, BOB, CHARLIE, 50)));
		});
}

#[test]
fn transfer_all_allow_death_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), true);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_ok!(Tokens::transfer_all(Some(ALICE).into(), CHARLIE, DOT, false));
			System::assert_last_event(Event::Tokens(crate::Event::Transfer(DOT, ALICE, CHARLIE, 100)));
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), false);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);

			assert_ok!(Tokens::set_lock(ID_1, DOT, &BOB, 50));
			assert_eq!(Tokens::accounts(&BOB, DOT).frozen, 50);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);
			assert_ok!(Tokens::transfer_all(Some(BOB).into(), CHARLIE, DOT, false));
			System::assert_last_event(Event::Tokens(crate::Event::Transfer(DOT, BOB, CHARLIE, 50)));
		});
}

#[test]
fn force_transfer_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), true);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);
			assert_noop!(
				Tokens::force_transfer(Some(ALICE).into(), ALICE, BOB, DOT, 100),
				BadOrigin
			);

			// imply AllowDeath
			assert_ok!(Tokens::force_transfer(RawOrigin::Root.into(), ALICE, BOB, DOT, 100));
			System::assert_last_event(Event::Tokens(crate::Event::Transfer(DOT, ALICE, BOB, 100)));
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), false);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 200);
		});
}

#[test]
fn set_balance_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
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

			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), true);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::total_issuance(DOT), 202);

			assert_ok!(Tokens::set_balance(RawOrigin::Root.into(), ALICE, DOT, 200, 100));
			System::assert_has_event(Event::Tokens(crate::Event::BalanceSet(DOT, ALICE, 200, 100)));
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), true);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 200);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::total_issuance(DOT), 402);

			assert_eq!(Accounts::<Runtime>::contains_key(BOB, DOT), true);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);
			assert_eq!(Tokens::reserved_balance(DOT, &BOB), 0);

			assert_ok!(Tokens::set_balance(RawOrigin::Root.into(), BOB, DOT, 0, 0));
			System::assert_has_event(Event::Tokens(crate::Event::BalanceSet(DOT, BOB, 0, 0)));
			assert_eq!(Accounts::<Runtime>::contains_key(BOB, DOT), false);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 0);
			assert_eq!(Tokens::reserved_balance(DOT, &BOB), 0);
			assert_eq!(Tokens::total_issuance(DOT), 302);

			assert_eq!(Accounts::<Runtime>::contains_key(CHARLIE, DOT), false);
			assert_eq!(Tokens::free_balance(DOT, &CHARLIE), 0);
			assert_eq!(Tokens::reserved_balance(DOT, &CHARLIE), 0);

			// below ED,
			assert_ok!(Tokens::set_balance(RawOrigin::Root.into(), CHARLIE, DOT, 1, 0));
			System::assert_has_event(Event::Tokens(crate::Event::BalanceSet(DOT, CHARLIE, 0, 0)));
			assert_eq!(Accounts::<Runtime>::contains_key(CHARLIE, DOT), false);
			assert_eq!(Tokens::free_balance(DOT, &CHARLIE), 0);
			assert_eq!(Tokens::reserved_balance(DOT, &CHARLIE), 0);
			assert_eq!(Tokens::total_issuance(DOT), 302);
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
		assert_eq!(System::can_dec_provider(&ALICE), false);
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
		assert_eq!(System::can_dec_provider(&ALICE), true);
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
		.one_hundred_for_alice_n_bob()
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
		// set_free_balance do not update total inssurance!
		assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
		assert_eq!(Tokens::total_issuance(DOT), 2);
		Tokens::set_free_balance(DOT, &ALICE, 100);
		assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
		assert_eq!(Tokens::total_issuance(DOT), 2);
	});
}

#[test]
fn set_reserved_balance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// set_reserved_balance do not update total inssurance!
		assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 0);
		assert_eq!(Tokens::total_issuance(DOT), 2);
		Tokens::set_reserved_balance(DOT, &ALICE, 100);
		assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 100);
		assert_eq!(Tokens::total_issuance(DOT), 2);
	});
}

#[test]
fn do_transfer_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
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
fn do_transfer_failed_due_to_keep_alive() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_noop!(
				Tokens::do_transfer(DOT, &ALICE, &BOB, 99, ExistenceRequirement::KeepAlive),
				Error::<Runtime>::KeepAlive
			);
		});
}

#[test]
fn do_transfer_and_dust_remove_when_allow_death() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);
			assert_eq!(Tokens::free_balance(DOT, &DustAccount::get()), 2);

			assert_ok!(Tokens::do_transfer(
				DOT,
				&ALICE,
				&BOB,
				99,
				ExistenceRequirement::AllowDeath
			));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 199);
			assert_eq!(Tokens::free_balance(DOT, &DustAccount::get()), 3);
		});
}

#[test]
fn do_transfer_failed_when_allow_death_due_to_cannot_dec_provider() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(System::can_dec_provider(&ALICE), true);
			assert_ok!(System::inc_consumers(&ALICE));
			assert_eq!(System::can_dec_provider(&ALICE), false);
			assert_noop!(
				Tokens::do_transfer(DOT, &ALICE, &BOB, 99, ExistenceRequirement::AllowDeath),
				Error::<Runtime>::KeepAlive
			);

			assert_ok!(Tokens::deposit(BTC, &ALICE, 100));
			assert_eq!(System::can_dec_provider(&ALICE), true);
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
fn do_withdraw_when_keep_alive() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::total_issuance(DOT), 202);

			assert_ok!(Tokens::do_withdraw(
				DOT,
				&ALICE,
				50,
				ExistenceRequirement::KeepAlive,
				true
			));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(DOT), 152);

			assert_noop!(
				Tokens::do_withdraw(DOT, &ALICE, 49, ExistenceRequirement::KeepAlive, true),
				Error::<Runtime>::KeepAlive
			);

			// do not change issuance
			assert_ok!(Tokens::do_withdraw(
				DOT,
				&ALICE,
				10,
				ExistenceRequirement::KeepAlive,
				false
			));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 40);
			assert_eq!(Tokens::total_issuance(DOT), 152);

			assert_ok!(Tokens::set_lock(ID_1, DOT, &BOB, 50));
			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);
			assert_noop!(
				Tokens::do_withdraw(DOT, &BOB, 51, ExistenceRequirement::KeepAlive, true),
				Error::<Runtime>::LiquidityRestrictions
			);
		});
}

#[test]
fn do_withdraw_when_allow_death() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::total_issuance(DOT), 202);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), true);

			assert_ok!(Tokens::do_withdraw(
				DOT,
				&ALICE,
				99,
				ExistenceRequirement::AllowDeath,
				true
			));
			assert_eq!(Tokens::total_issuance(DOT), 103);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), false);
		});
}

#[test]
fn do_deposit_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Accounts::<Runtime>::contains_key(CHARLIE, DOT), false);
			assert_eq!(Tokens::free_balance(DOT, &CHARLIE), 0);
			assert_eq!(Tokens::total_issuance(DOT), 202);
			assert_noop!(
				Tokens::do_deposit(DOT, &CHARLIE, 10, true, true),
				Error::<Runtime>::DeadAccount
			);

			assert_noop!(
				Tokens::do_deposit(DOT, &CHARLIE, 1, false, true),
				Error::<Runtime>::ExistentialDeposit
			);

			assert_ok!(Tokens::do_deposit(DOT, &CHARLIE, 10, false, true));
			assert_eq!(Accounts::<Runtime>::contains_key(CHARLIE, DOT), true);
			assert_eq!(Tokens::free_balance(DOT, &CHARLIE), 10);
			assert_eq!(Tokens::total_issuance(DOT), 212);

			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), true);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);

			assert_ok!(Tokens::do_deposit(DOT, &ALICE, 10, true, true));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 110);
			assert_eq!(Tokens::total_issuance(DOT), 222);

			assert_noop!(
				Tokens::do_deposit(DOT, &BOB, Balance::max_value(), false, true),
				ArithmeticError::Overflow
			);

			// do not change issuance
			assert_ok!(Tokens::do_deposit(DOT, &ALICE, 10, true, false));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 120);
			assert_eq!(Tokens::total_issuance(DOT), 222);
		});
}

// *************************************************
// tests for endowed account and remove account
// *************************************************

#[test]
fn endowed_account_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(System::providers(&ALICE), 0);
		assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), false);
		Tokens::set_free_balance(DOT, &ALICE, 100);
		System::assert_last_event(Event::Tokens(crate::Event::Endowed(DOT, ALICE, 100)));
		assert_eq!(System::providers(&ALICE), 1);
		assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), true);
	});
}

#[test]
fn remove_account_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(System::providers(&ALICE), 1);
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), true);
			Tokens::set_free_balance(DOT, &ALICE, 0);
			assert_eq!(System::providers(&ALICE), 0);
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), false);
		});
}

#[test]
fn dust_remove_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			let dust_account = DustAccount::get();

			assert_eq!(System::providers(&ALICE), 1);
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), true);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::free_balance(DOT, &dust_account), 2);
			Tokens::set_free_balance(DOT, &ALICE, 1);
			System::assert_last_event(Event::Tokens(crate::Event::DustLost(DOT, ALICE, 1)));
			assert_eq!(System::providers(&ALICE), 0);
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), false);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::free_balance(DOT, &dust_account), 3);
		});
}

#[test]
fn account_survive_due_to_dust_transfer_failure() {
	ExtBuilder::default().build().execute_with(|| {
		let dust_account = DustAccount::get();

		Tokens::set_free_balance(DOT, &dust_account, 0);
		assert_eq!(Tokens::free_balance(DOT, &dust_account), 0);
		assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
		assert_eq!(System::providers(&ALICE), 0);
		assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), false);
		Tokens::set_free_balance(DOT, &ALICE, 1);
		System::assert_last_event(Event::Tokens(crate::Event::DustLost(DOT, ALICE, 1)));
		assert_eq!(Tokens::free_balance(DOT, &dust_account), 0);
		assert_eq!(Tokens::free_balance(DOT, &ALICE), 1);
		assert_eq!(System::providers(&ALICE), 1);
		assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), true);
	});
}

// *************************************************
// tests for MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency,
// MultiReservableCurrency traits **********************************************
// ***

#[test]
fn multicurrency_deposit_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Accounts::<Runtime>::contains_key(CHARLIE, DOT), false);
		assert_eq!(Tokens::free_balance(DOT, &CHARLIE), 0);
		assert_eq!(Tokens::total_issuance(DOT), 2);
		assert_ok!(Tokens::deposit(DOT, &CHARLIE, 10));
		assert_eq!(Accounts::<Runtime>::contains_key(CHARLIE, DOT), true);
		assert_eq!(Tokens::free_balance(DOT, &CHARLIE), 10);
		assert_eq!(Tokens::total_issuance(DOT), 12);
	});
}

#[test]
fn multicurrency_withdraw_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), true);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::total_issuance(DOT), 202);
			assert_ok!(Tokens::withdraw(DOT, &ALICE, 99));
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), false);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::total_issuance(DOT), 103);
		});
}

#[test]
fn multicurrency_transfer_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), true);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 100);
			assert_ok!(<Tokens as MultiCurrency<_>>::transfer(DOT, &ALICE, &BOB, 99));
			assert_eq!(Accounts::<Runtime>::contains_key(ALICE, DOT), false);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 199);
		});
}

#[test]
fn multicurrency_can_slash_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::can_slash(DOT, &ALICE, 101), false);
			assert_eq!(Tokens::can_slash(DOT, &ALICE, 100), true);
		});
}

#[test]
fn multicurrency_slash_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			// slashed_amount < amount
			assert_eq!(Tokens::total_issuance(DOT), 202);
			assert_eq!(Tokens::slash(DOT, &ALICE, 50), 0);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(DOT), 152);

			// slashed_amount == amount
			assert_eq!(Tokens::slash(DOT, &ALICE, 51), 1);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::total_issuance(DOT), 102);
		});
}

#[test]
fn multicurrency_extended_update_balance_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::update_balance(DOT, &ALICE, 50));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 150);
			assert_eq!(Tokens::total_issuance(DOT), 252);

			assert_ok!(Tokens::update_balance(DOT, &BOB, -50));
			assert_eq!(Tokens::free_balance(DOT, &BOB), 50);
			assert_eq!(Tokens::total_issuance(DOT), 202);

			assert_noop!(Tokens::update_balance(DOT, &BOB, -60), Error::<Runtime>::BalanceTooLow);
		});
}

#[test]
fn multi_lockable_currency_set_lock_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
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
		.one_hundred_for_alice_n_bob()
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
		.one_hundred_for_alice_n_bob()
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
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::can_reserve(DOT, &ALICE, 0), true);
			assert_eq!(Tokens::can_reserve(DOT, &ALICE, 101), false);
			assert_eq!(Tokens::can_reserve(DOT, &ALICE, 100), true);
		});
}

#[test]
fn multi_reservable_currency_slash_reserved_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::reserve(DOT, &ALICE, 50));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(DOT), 202);
			assert_eq!(Tokens::slash_reserved(DOT, &ALICE, 0), 0);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(DOT), 202);
			assert_eq!(Tokens::slash_reserved(DOT, &ALICE, 100), 50);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::total_issuance(DOT), 152);
		});
}

#[test]
fn multi_reservable_currency_reserve_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_noop!(Tokens::reserve(DOT, &ALICE, 101), Error::<Runtime>::BalanceTooLow);
			assert_ok!(Tokens::reserve(DOT, &ALICE, 0));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::total_balance(DOT, &ALICE), 100);
			assert_ok!(Tokens::reserve(DOT, &ALICE, 50));
			System::assert_last_event(Event::Tokens(crate::Event::Reserved(DOT, ALICE, 50)));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::total_balance(DOT, &ALICE), 100);
		});
}

#[test]
fn multi_reservable_currency_unreserve_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::unreserve(DOT, &ALICE, 0), 0);
			assert_eq!(Tokens::unreserve(DOT, &ALICE, 50), 50);
			System::assert_last_event(Event::Tokens(crate::Event::Unreserved(DOT, ALICE, 0)));
			assert_ok!(Tokens::reserve(DOT, &ALICE, 30));
			System::assert_last_event(Event::Tokens(crate::Event::Reserved(DOT, ALICE, 30)));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 70);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 30);
			assert_eq!(Tokens::unreserve(DOT, &ALICE, 15), 0);
			System::assert_last_event(Event::Tokens(crate::Event::Unreserved(DOT, ALICE, 15)));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 85);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 15);
			assert_eq!(Tokens::unreserve(DOT, &ALICE, 30), 15);
			System::assert_last_event(Event::Tokens(crate::Event::Unreserved(DOT, ALICE, 15)));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 0);
		});
}

#[test]
fn multi_reservable_currency_repatriate_reserved_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
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
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 30);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 50);
			assert_eq!(Tokens::reserved_balance(DOT, &BOB), 20);

			assert_eq!(
				Tokens::repatriate_reserved(DOT, &BOB, &ALICE, 30, BalanceStatus::Free),
				Ok(10)
			);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 120);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 30);
			assert_eq!(Tokens::free_balance(DOT, &BOB), 50);
			assert_eq!(Tokens::reserved_balance(DOT, &BOB), 0);
		});
}

#[test]
fn slash_draw_reserved_correct() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::reserve(DOT, &ALICE, 50));
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(DOT), 202);

			assert_eq!(Tokens::slash(DOT, &ALICE, 80), 0);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 20);
			assert_eq!(Tokens::total_issuance(DOT), 122);

			assert_eq!(Tokens::slash(DOT, &ALICE, 50), 30);
			assert_eq!(Tokens::free_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::reserved_balance(DOT, &ALICE), 0);
			assert_eq!(Tokens::total_issuance(DOT), 102);
		});
}

#[test]
fn no_op_if_amount_is_zero() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Tokens::ensure_can_withdraw(DOT, &ALICE, 0));
		assert_ok!(Tokens::transfer(Some(ALICE).into(), BOB, DOT, 0));
		assert_ok!(Tokens::transfer(Some(ALICE).into(), ALICE, DOT, 0));
		assert_ok!(Tokens::deposit(DOT, &ALICE, 0));
		assert_ok!(Tokens::withdraw(DOT, &ALICE, 0));
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

// *************************************************
// tests for CurrencyAdapter
// *************************************************

#[test]
fn currency_adapter_ensure_currency_adapter_should_work() {
	ExtBuilder::default()
		.one_hundred_for_treasury_account()
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::total_issuance(DOT), 104);
			assert_eq!(Tokens::total_balance(DOT, &Treasury::account_id()), 2);
			assert_eq!(Tokens::total_balance(DOT, &TREASURY_ACCOUNT), 100);
			assert_eq!(Tokens::reserved_balance(DOT, &TREASURY_ACCOUNT), 0);
			assert_eq!(Tokens::free_balance(DOT, &TREASURY_ACCOUNT), 100);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::total_balance(&TREASURY_ACCOUNT),
				100
			);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::can_slash(&TREASURY_ACCOUNT, 10),
				true
			);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::total_issuance(),
				104
			);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::minimum_balance(),
				2
			);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::can_reserve(&TREASURY_ACCOUNT, 5),
				true
			);

			// burn
			let imbalance = <Runtime as pallet_elections_phragmen::Config>::Currency::burn(10);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::total_issuance(),
				94
			);
			drop(imbalance);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::total_issuance(),
				104
			);

			// issue
			let imbalance = <Runtime as pallet_elections_phragmen::Config>::Currency::issue(20);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::total_issuance(),
				124
			);
			drop(imbalance);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::total_issuance(),
				104
			);

			// transfer
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::free_balance(&TREASURY_ACCOUNT),
				100
			);
			assert_ok!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::ensure_can_withdraw(
					&TREASURY_ACCOUNT,
					10,
					WithdrawReasons::TRANSFER,
					0
				)
			);
			assert_ok!(<Runtime as pallet_elections_phragmen::Config>::Currency::transfer(
				&TREASURY_ACCOUNT,
				&ALICE,
				11,
				ExistenceRequirement::KeepAlive
			));
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::free_balance(&TREASURY_ACCOUNT),
				89
			);

			// deposit
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::total_issuance(),
				104
			);
			let imbalance = TreasuryCurrencyAdapter::deposit_creating(&TREASURY_ACCOUNT, 11);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::free_balance(&TREASURY_ACCOUNT),
				100
			);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::total_issuance(),
				104
			);
			drop(imbalance);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::free_balance(&TREASURY_ACCOUNT),
				100
			);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::total_issuance(),
				115
			);

			// withdraw
			let imbalance = <Runtime as pallet_elections_phragmen::Config>::Currency::withdraw(
				&TREASURY_ACCOUNT,
				10,
				WithdrawReasons::TRANSFER,
				ExistenceRequirement::KeepAlive,
			);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::free_balance(&TREASURY_ACCOUNT),
				90
			);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::total_issuance(),
				115
			);
			drop(imbalance);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::free_balance(&TREASURY_ACCOUNT),
				90
			);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Config>::Currency::total_issuance(),
				105
			);
		});
}

#[test]
fn currency_adapter_burn_must_work() {
	ExtBuilder::default()
		.one_hundred_for_treasury_account()
		.build()
		.execute_with(|| {
			let init_total_issuance = TreasuryCurrencyAdapter::total_issuance();
			let imbalance = TreasuryCurrencyAdapter::burn(10);
			assert_eq!(TreasuryCurrencyAdapter::total_issuance(), init_total_issuance - 10);
			drop(imbalance);
			assert_eq!(TreasuryCurrencyAdapter::total_issuance(), init_total_issuance);
		});
}

#[test]
fn currency_adapter_reserving_balance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let _ = TreasuryCurrencyAdapter::deposit_creating(&TREASURY_ACCOUNT, 111);

		assert_eq!(TreasuryCurrencyAdapter::total_balance(&TREASURY_ACCOUNT), 111);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT), 111);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&TREASURY_ACCOUNT), 0);

		assert_ok!(TreasuryCurrencyAdapter::reserve(&TREASURY_ACCOUNT, 69));

		assert_eq!(TreasuryCurrencyAdapter::total_balance(&TREASURY_ACCOUNT), 111);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT), 42);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&TREASURY_ACCOUNT), 69);
	});
}

#[test]
fn currency_adapter_balance_transfer_when_reserved_should_not_work() {
	ExtBuilder::default().build().execute_with(|| {
		let _ = TreasuryCurrencyAdapter::deposit_creating(&TREASURY_ACCOUNT, 111);
		assert_ok!(TreasuryCurrencyAdapter::reserve(&TREASURY_ACCOUNT, 69));
		assert_noop!(
			TreasuryCurrencyAdapter::transfer(&TREASURY_ACCOUNT, &ALICE, 69, ExistenceRequirement::AllowDeath),
			Error::<Runtime>::BalanceTooLow,
		);
	});
}

#[test]
fn currency_adapter_deducting_balance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let _ = TreasuryCurrencyAdapter::deposit_creating(&TREASURY_ACCOUNT, 111);
		assert_ok!(TreasuryCurrencyAdapter::reserve(&TREASURY_ACCOUNT, 69));
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT), 42);
	});
}

#[test]
fn currency_adapter_refunding_balance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let _ = TreasuryCurrencyAdapter::deposit_creating(&TREASURY_ACCOUNT, 42);
		Tokens::set_reserved_balance(DOT, &TREASURY_ACCOUNT, 69);
		TreasuryCurrencyAdapter::unreserve(&TREASURY_ACCOUNT, 69);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT), 111);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&TREASURY_ACCOUNT), 0);
	});
}

#[test]
fn currency_adapter_slashing_balance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let _ = TreasuryCurrencyAdapter::deposit_creating(&TREASURY_ACCOUNT, 111);
		assert_ok!(TreasuryCurrencyAdapter::reserve(&TREASURY_ACCOUNT, 69));
		assert!(TreasuryCurrencyAdapter::slash(&TREASURY_ACCOUNT, 69).1.is_zero());
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT), 0);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&TREASURY_ACCOUNT), 42);
		assert_eq!(TreasuryCurrencyAdapter::total_issuance(), 44);
	});
}

#[test]
fn currency_adapter_slashing_incomplete_balance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let _ = TreasuryCurrencyAdapter::deposit_creating(&TREASURY_ACCOUNT, 42);
		assert_eq!(TreasuryCurrencyAdapter::total_issuance(), 44);
		assert_ok!(TreasuryCurrencyAdapter::reserve(&TREASURY_ACCOUNT, 21));
		assert_eq!(TreasuryCurrencyAdapter::slash(&TREASURY_ACCOUNT, 69).1, 27);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT), 0);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&TREASURY_ACCOUNT), 0);
		assert_eq!(TreasuryCurrencyAdapter::total_issuance(), 2);
	});
}

#[test]
fn currency_adapter_basic_locking_should_work() {
	ExtBuilder::default()
		.one_hundred_for_treasury_account()
		.build()
		.execute_with(|| {
			assert_eq!(TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT), 100);
			TreasuryCurrencyAdapter::set_lock(ID_1, &TREASURY_ACCOUNT, 91, WithdrawReasons::all());
			assert_noop!(
				TreasuryCurrencyAdapter::transfer(&TREASURY_ACCOUNT, &ALICE, 10, ExistenceRequirement::AllowDeath),
				Error::<Runtime>::LiquidityRestrictions
			);
		});
}

#[test]
fn currency_adapter_partial_locking_should_work() {
	ExtBuilder::default()
		.one_hundred_for_treasury_account()
		.build()
		.execute_with(|| {
			TreasuryCurrencyAdapter::set_lock(ID_1, &TREASURY_ACCOUNT, 5, WithdrawReasons::all());
			assert_ok!(TreasuryCurrencyAdapter::transfer(
				&TREASURY_ACCOUNT,
				&ALICE,
				2,
				ExistenceRequirement::AllowDeath
			));
		});
}

#[test]
fn currency_adapter_lock_removal_should_work() {
	ExtBuilder::default()
		.one_hundred_for_treasury_account()
		.build()
		.execute_with(|| {
			TreasuryCurrencyAdapter::set_lock(ID_1, &TREASURY_ACCOUNT, u64::max_value(), WithdrawReasons::all());
			TreasuryCurrencyAdapter::remove_lock(ID_1, &TREASURY_ACCOUNT);
			assert_ok!(TreasuryCurrencyAdapter::transfer(
				&TREASURY_ACCOUNT,
				&ALICE,
				2,
				ExistenceRequirement::AllowDeath
			));
		});
}

#[test]
fn currency_adapter_lock_replacement_should_work() {
	ExtBuilder::default()
		.one_hundred_for_treasury_account()
		.build()
		.execute_with(|| {
			TreasuryCurrencyAdapter::set_lock(ID_1, &TREASURY_ACCOUNT, u64::max_value(), WithdrawReasons::all());
			TreasuryCurrencyAdapter::set_lock(ID_1, &TREASURY_ACCOUNT, 5, WithdrawReasons::all());
			assert_ok!(TreasuryCurrencyAdapter::transfer(
				&TREASURY_ACCOUNT,
				&ALICE,
				2,
				ExistenceRequirement::AllowDeath
			));
		});
}

#[test]
fn currency_adapter_double_locking_should_work() {
	ExtBuilder::default()
		.one_hundred_for_treasury_account()
		.build()
		.execute_with(|| {
			TreasuryCurrencyAdapter::set_lock(ID_1, &TREASURY_ACCOUNT, 5, WithdrawReasons::empty());
			TreasuryCurrencyAdapter::set_lock(ID_2, &TREASURY_ACCOUNT, 5, WithdrawReasons::all());
			assert_ok!(TreasuryCurrencyAdapter::transfer(
				&TREASURY_ACCOUNT,
				&ALICE,
				2,
				ExistenceRequirement::AllowDeath
			));
		});
}

#[test]
fn currency_adapter_combination_locking_should_work() {
	ExtBuilder::default()
		.one_hundred_for_treasury_account()
		.build()
		.execute_with(|| {
			// withdrawReasons not work
			TreasuryCurrencyAdapter::set_lock(ID_1, &TREASURY_ACCOUNT, u64::max_value(), WithdrawReasons::empty());
			TreasuryCurrencyAdapter::set_lock(ID_2, &TREASURY_ACCOUNT, 0, WithdrawReasons::all());
			assert_noop!(
				TreasuryCurrencyAdapter::transfer(&TREASURY_ACCOUNT, &ALICE, 2, ExistenceRequirement::AllowDeath),
				Error::<Runtime>::LiquidityRestrictions
			);
		});
}

#[test]
fn currency_adapter_lock_value_extension_should_work() {
	ExtBuilder::default()
		.one_hundred_for_treasury_account()
		.build()
		.execute_with(|| {
			TreasuryCurrencyAdapter::set_lock(ID_1, &TREASURY_ACCOUNT, 100, WithdrawReasons::all());
			assert_noop!(
				TreasuryCurrencyAdapter::transfer(&TREASURY_ACCOUNT, &ALICE, 6, ExistenceRequirement::AllowDeath),
				Error::<Runtime>::LiquidityRestrictions
			);
			TreasuryCurrencyAdapter::extend_lock(ID_1, &TREASURY_ACCOUNT, 2, WithdrawReasons::all());
			assert_noop!(
				TreasuryCurrencyAdapter::transfer(&TREASURY_ACCOUNT, &ALICE, 6, ExistenceRequirement::AllowDeath),
				Error::<Runtime>::LiquidityRestrictions
			);
			TreasuryCurrencyAdapter::extend_lock(ID_1, &TREASURY_ACCOUNT, 8, WithdrawReasons::all());
			assert_noop!(
				TreasuryCurrencyAdapter::transfer(&TREASURY_ACCOUNT, &ALICE, 3, ExistenceRequirement::AllowDeath),
				Error::<Runtime>::LiquidityRestrictions
			);
		});
}

#[test]
fn currency_adapter_lock_block_number_extension_should_work() {
	ExtBuilder::default()
		.one_hundred_for_treasury_account()
		.build()
		.execute_with(|| {
			TreasuryCurrencyAdapter::set_lock(ID_1, &TREASURY_ACCOUNT, 200, WithdrawReasons::all());
			assert_noop!(
				TreasuryCurrencyAdapter::transfer(&TREASURY_ACCOUNT, &ALICE, 6, ExistenceRequirement::AllowDeath),
				Error::<Runtime>::LiquidityRestrictions
			);
			TreasuryCurrencyAdapter::extend_lock(ID_1, &TREASURY_ACCOUNT, 90, WithdrawReasons::all());
			assert_noop!(
				TreasuryCurrencyAdapter::transfer(&TREASURY_ACCOUNT, &ALICE, 6, ExistenceRequirement::AllowDeath),
				Error::<Runtime>::LiquidityRestrictions
			);
			System::set_block_number(2);
			TreasuryCurrencyAdapter::extend_lock(ID_1, &TREASURY_ACCOUNT, 90, WithdrawReasons::all());
			assert_noop!(
				TreasuryCurrencyAdapter::transfer(&TREASURY_ACCOUNT, &ALICE, 3, ExistenceRequirement::AllowDeath),
				Error::<Runtime>::LiquidityRestrictions
			);
		});
}

#[test]
fn currency_adapter_lock_reasons_extension_should_work() {
	ExtBuilder::default()
		.one_hundred_for_treasury_account()
		.build()
		.execute_with(|| {
			TreasuryCurrencyAdapter::set_lock(ID_1, &TREASURY_ACCOUNT, 90, WithdrawReasons::TRANSFER);
			assert_noop!(
				TreasuryCurrencyAdapter::transfer(&TREASURY_ACCOUNT, &ALICE, 11, ExistenceRequirement::AllowDeath),
				Error::<Runtime>::LiquidityRestrictions
			);
			TreasuryCurrencyAdapter::extend_lock(ID_1, &TREASURY_ACCOUNT, 90, WithdrawReasons::empty());
			assert_noop!(
				TreasuryCurrencyAdapter::transfer(&TREASURY_ACCOUNT, &ALICE, 11, ExistenceRequirement::AllowDeath),
				Error::<Runtime>::LiquidityRestrictions
			);
			TreasuryCurrencyAdapter::extend_lock(ID_1, &TREASURY_ACCOUNT, 90, WithdrawReasons::RESERVE);
			assert_noop!(
				TreasuryCurrencyAdapter::transfer(&TREASURY_ACCOUNT, &ALICE, 11, ExistenceRequirement::AllowDeath),
				Error::<Runtime>::LiquidityRestrictions
			);
		});
}

#[test]
fn currency_adapter_deposit_creating_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(TreasuryCurrencyAdapter::total_issuance(), 202);
			assert_eq!(TreasuryCurrencyAdapter::total_balance(&TREASURY_ACCOUNT), 0);
			let _ = TreasuryCurrencyAdapter::deposit_creating(&TREASURY_ACCOUNT, 2);
			assert_eq!(TreasuryCurrencyAdapter::total_issuance(), 204);
			assert_eq!(TreasuryCurrencyAdapter::total_balance(&TREASURY_ACCOUNT), 2);

			assert_eq!(TreasuryCurrencyAdapter::total_balance(&ALICE), 100);
			let _ = TreasuryCurrencyAdapter::deposit_creating(&ALICE, 1);
			assert_eq!(TreasuryCurrencyAdapter::total_issuance(), 205);
			assert_eq!(TreasuryCurrencyAdapter::total_balance(&ALICE), 101);
		});
}

#[test]
fn currency_adapter_deposit_into_existing_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(TreasuryCurrencyAdapter::total_balance(&TREASURY_ACCOUNT), 0);
			assert_noop!(
				TreasuryCurrencyAdapter::deposit_into_existing(&TREASURY_ACCOUNT, 10).map(drop),
				Error::<Runtime>::DeadAccount,
			);

			assert_eq!(TreasuryCurrencyAdapter::total_issuance(), 202);
			assert_eq!(TreasuryCurrencyAdapter::total_balance(&ALICE), 100);
			assert_ok!(TreasuryCurrencyAdapter::deposit_into_existing(&ALICE, 10).map(drop));
			assert_eq!(TreasuryCurrencyAdapter::total_issuance(), 212);
			assert_eq!(TreasuryCurrencyAdapter::total_balance(&ALICE), 110);
		});
}

#[test]
fn currency_adapter_reward_should_work() {
	ExtBuilder::default()
		.one_hundred_for_treasury_account()
		.build()
		.execute_with(|| {
			assert_eq!(TreasuryCurrencyAdapter::total_issuance(), 104);
			assert_eq!(TreasuryCurrencyAdapter::total_balance(&TREASURY_ACCOUNT), 100);
			assert_eq!(TreasuryCurrencyAdapter::total_balance(&Treasury::account_id()), 2);
			assert_ok!(TreasuryCurrencyAdapter::deposit_into_existing(&TREASURY_ACCOUNT, 10).map(drop));
			assert_eq!(TreasuryCurrencyAdapter::total_balance(&TREASURY_ACCOUNT), 110);
			assert_eq!(TreasuryCurrencyAdapter::total_issuance(), 114);
		});
}

#[test]
fn currency_adapter_slashing_reserved_balance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let _ = TreasuryCurrencyAdapter::deposit_creating(&TREASURY_ACCOUNT, 111);
		assert_eq!(TreasuryCurrencyAdapter::total_issuance(), 113);
		assert_ok!(TreasuryCurrencyAdapter::reserve(&TREASURY_ACCOUNT, 111));
		assert_eq!(TreasuryCurrencyAdapter::slash_reserved(&TREASURY_ACCOUNT, 42).1, 0);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&TREASURY_ACCOUNT), 69);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT), 0);
		assert_eq!(TreasuryCurrencyAdapter::total_issuance(), 71);
	});
}

#[test]
fn currency_adapter_slashing_incomplete_reserved_balance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let _ = TreasuryCurrencyAdapter::deposit_creating(&TREASURY_ACCOUNT, 111);
		assert_eq!(TreasuryCurrencyAdapter::total_issuance(), 113);
		assert_ok!(TreasuryCurrencyAdapter::reserve(&TREASURY_ACCOUNT, 42));
		assert_eq!(TreasuryCurrencyAdapter::slash_reserved(&TREASURY_ACCOUNT, 69).1, 27);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT), 69);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&TREASURY_ACCOUNT), 0);
		assert_eq!(TreasuryCurrencyAdapter::total_issuance(), 71);
	});
}

#[test]
fn currency_adapter_repatriating_reserved_balance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let _ = TreasuryCurrencyAdapter::deposit_creating(&TREASURY_ACCOUNT, 110);
		let _ = TreasuryCurrencyAdapter::deposit_creating(&ALICE, 2);
		assert_ok!(TreasuryCurrencyAdapter::reserve(&TREASURY_ACCOUNT, 110));
		assert_ok!(
			TreasuryCurrencyAdapter::repatriate_reserved(&TREASURY_ACCOUNT, &ALICE, 41, Status::Free),
			0
		);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&TREASURY_ACCOUNT), 69);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT), 0);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&ALICE), 0);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&ALICE), 43);
	});
}

#[test]
fn currency_adapter_transferring_reserved_balance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let _ = TreasuryCurrencyAdapter::deposit_creating(&TREASURY_ACCOUNT, 110);
		let _ = TreasuryCurrencyAdapter::deposit_creating(&ALICE, 2);
		assert_ok!(TreasuryCurrencyAdapter::reserve(&TREASURY_ACCOUNT, 110));
		assert_ok!(
			TreasuryCurrencyAdapter::repatriate_reserved(&TREASURY_ACCOUNT, &ALICE, 41, Status::Reserved),
			0
		);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&TREASURY_ACCOUNT), 69);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT), 0);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&ALICE), 41);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&ALICE), 2);
	});
}

#[test]
fn currency_adapter_transferring_reserved_balance_to_nonexistent_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let _ = TreasuryCurrencyAdapter::deposit_creating(&TREASURY_ACCOUNT, 111);
		assert_ok!(TreasuryCurrencyAdapter::reserve(&TREASURY_ACCOUNT, 111));
		assert_ok!(TreasuryCurrencyAdapter::repatriate_reserved(
			&TREASURY_ACCOUNT,
			&ALICE,
			42,
			Status::Free
		));
	});
}

#[test]
fn currency_adapter_transferring_incomplete_reserved_balance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let _ = TreasuryCurrencyAdapter::deposit_creating(&TREASURY_ACCOUNT, 110);
		let _ = TreasuryCurrencyAdapter::deposit_creating(&ALICE, 2);
		assert_ok!(TreasuryCurrencyAdapter::reserve(&TREASURY_ACCOUNT, 41));
		assert_ok!(
			TreasuryCurrencyAdapter::repatriate_reserved(&TREASURY_ACCOUNT, &ALICE, 69, Status::Free),
			28
		);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&TREASURY_ACCOUNT), 0);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT), 69);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&ALICE), 0);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&ALICE), 43);
	});
}

#[test]
fn currency_adapter_transferring_too_high_value_should_not_panic() {
	ExtBuilder::default().build().execute_with(|| {
		TreasuryCurrencyAdapter::make_free_balance_be(&TREASURY_ACCOUNT, u64::max_value());
		TreasuryCurrencyAdapter::make_free_balance_be(&ALICE, 2);

		assert_noop!(
			TreasuryCurrencyAdapter::transfer(
				&TREASURY_ACCOUNT,
				&ALICE,
				u64::max_value(),
				ExistenceRequirement::AllowDeath
			),
			ArithmeticError::Overflow,
		);

		assert_eq!(
			TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT),
			u64::max_value()
		);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&ALICE), 2);
	});
}

#[test]
fn exceeding_max_locks_should_fail() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::set_lock(ID_1, DOT, &ALICE, 10));
			assert_eq!(Tokens::locks(ALICE, DOT).len(), 1);
			assert_ok!(Tokens::set_lock(ID_2, DOT, &ALICE, 10));
			assert_eq!(Tokens::locks(ALICE, DOT).len(), 2);
			assert_noop!(
				Tokens::set_lock(ID_3, DOT, &ALICE, 10),
				Error::<Runtime>::MaxLocksExceeded
			);
			assert_eq!(Tokens::locks(ALICE, DOT).len(), 2);
		});
}

// *************************************************
// tests for fungibles traits
// *************************************************

#[test]
fn fungibles_inspect_trait_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(<Tokens as fungibles::Inspect<_>>::total_issuance(DOT), 202);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::minimum_balance(DOT), 2);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 100);
			assert_eq!(
				<Tokens as fungibles::Inspect<_>>::reducible_balance(DOT, &ALICE, true),
				98
			);
			assert_ok!(<Tokens as fungibles::Inspect<_>>::can_deposit(DOT, &ALICE, 1).into_result());
			assert_ok!(<Tokens as fungibles::Inspect<_>>::can_withdraw(DOT, &ALICE, 1).into_result());
		});
}

#[test]
fn fungibles_mutate_trait_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(<Tokens as fungibles::Mutate<_>>::mint_into(DOT, &ALICE, 10));
			assert_eq!(<Tokens as fungibles::Mutate<_>>::burn_from(DOT, &ALICE, 8), Ok(8));
		});
}

#[test]
fn fungibles_transfer_trait_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 100);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &BOB), 100);
			assert_ok!(<Tokens as fungibles::Transfer<_>>::transfer(
				DOT, &ALICE, &BOB, 10, true
			));
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 90);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &BOB), 110);
		});
}

#[test]
fn fungibles_unbalanced_trait_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 100);
			assert_ok!(<Tokens as fungibles::Unbalanced<_>>::set_balance(DOT, &ALICE, 10));
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 10);

			assert_eq!(<Tokens as fungibles::Inspect<_>>::total_issuance(DOT), 202);
			<Tokens as fungibles::Unbalanced<_>>::set_total_issuance(DOT, 10);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::total_issuance(DOT), 10);
		});
}

#[test]
fn fungibles_inspect_hold_trait_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, &ALICE), 0);
			assert_eq!(<Tokens as fungibles::InspectHold<_>>::can_hold(DOT, &ALICE, 50), true);
			assert_eq!(<Tokens as fungibles::InspectHold<_>>::can_hold(DOT, &ALICE, 100), false);
		});
}

#[test]
fn fungibles_mutate_hold_trait_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_noop!(
				<Tokens as fungibles::MutateHold<_>>::hold(DOT, &ALICE, 200),
				Error::<Runtime>::BalanceTooLow
			);
			assert_ok!(<Tokens as fungibles::MutateHold<_>>::hold(DOT, &ALICE, 100));
			assert_eq!(
				<Tokens as fungibles::MutateHold<_>>::release(DOT, &ALICE, 50, true),
				Ok(50)
			);
			assert_eq!(
				<Tokens as fungibles::MutateHold<_>>::transfer_held(DOT, &ALICE, &BOB, 100, true, true),
				Ok(50)
			);
		});
}
