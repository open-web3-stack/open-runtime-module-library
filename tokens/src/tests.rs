//! Unit tests for the tokens module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{
	Balance, ExtBuilder, MockDustRemoval, Runtime, System, TestEvent, Tokens, ALICE, BOB, CHARLIE, ID_1, ID_2,
	TEST_TOKEN_ID,
};

#[test]
fn set_lock_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			Tokens::set_lock(ID_1, TEST_TOKEN_ID, &ALICE, 10);
			assert_eq!(Tokens::accounts(TEST_TOKEN_ID, &ALICE).frozen, 10);
			assert_eq!(Tokens::accounts(TEST_TOKEN_ID, &ALICE).frozen(), 10);
			assert_eq!(Tokens::locks(TEST_TOKEN_ID, ALICE).len(), 1);
			Tokens::set_lock(ID_1, TEST_TOKEN_ID, &ALICE, 50);
			assert_eq!(Tokens::accounts(TEST_TOKEN_ID, &ALICE).frozen, 50);
			assert_eq!(Tokens::locks(TEST_TOKEN_ID, ALICE).len(), 1);
			Tokens::set_lock(ID_2, TEST_TOKEN_ID, &ALICE, 60);
			assert_eq!(Tokens::accounts(TEST_TOKEN_ID, &ALICE).frozen, 60);
			assert_eq!(Tokens::locks(TEST_TOKEN_ID, ALICE).len(), 2);
		});
}

#[test]
fn extend_lock_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			Tokens::set_lock(ID_1, TEST_TOKEN_ID, &ALICE, 10);
			assert_eq!(Tokens::locks(TEST_TOKEN_ID, ALICE).len(), 1);
			assert_eq!(Tokens::accounts(TEST_TOKEN_ID, &ALICE).frozen, 10);
			Tokens::extend_lock(ID_1, TEST_TOKEN_ID, &ALICE, 20);
			assert_eq!(Tokens::locks(TEST_TOKEN_ID, ALICE).len(), 1);
			assert_eq!(Tokens::accounts(TEST_TOKEN_ID, &ALICE).frozen, 20);
			Tokens::extend_lock(ID_2, TEST_TOKEN_ID, &ALICE, 10);
			Tokens::extend_lock(ID_1, TEST_TOKEN_ID, &ALICE, 20);
			assert_eq!(Tokens::locks(TEST_TOKEN_ID, ALICE).len(), 2);
		});
}

#[test]
fn remove_lock_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			Tokens::set_lock(ID_1, TEST_TOKEN_ID, &ALICE, 10);
			Tokens::set_lock(ID_2, TEST_TOKEN_ID, &ALICE, 20);
			assert_eq!(Tokens::locks(TEST_TOKEN_ID, ALICE).len(), 2);
			Tokens::remove_lock(ID_2, TEST_TOKEN_ID, &ALICE);
			assert_eq!(Tokens::locks(TEST_TOKEN_ID, ALICE).len(), 1);
		});
}

#[test]
fn frozen_can_limit_liquidity() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			Tokens::set_lock(ID_1, TEST_TOKEN_ID, &ALICE, 90);
			assert_noop!(
				<Tokens as MultiCurrency<_>>::transfer(TEST_TOKEN_ID, &ALICE, &BOB, 11),
				Error::<Runtime>::LiquidityRestrictions,
			);
			Tokens::set_lock(ID_1, TEST_TOKEN_ID, &ALICE, 10);
			assert_ok!(<Tokens as MultiCurrency<_>>::transfer(TEST_TOKEN_ID, &ALICE, &BOB, 11),);
		});
}

#[test]
fn can_reserve_is_correct() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::can_reserve(TEST_TOKEN_ID, &ALICE, 0), true);
			assert_eq!(Tokens::can_reserve(TEST_TOKEN_ID, &ALICE, 101), false);
			assert_eq!(Tokens::can_reserve(TEST_TOKEN_ID, &ALICE, 100), true);
		});
}

#[test]
fn reserve_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_noop!(
				Tokens::reserve(TEST_TOKEN_ID, &ALICE, 101),
				Error::<Runtime>::BalanceTooLow,
			);
			assert_ok!(Tokens::reserve(TEST_TOKEN_ID, &ALICE, 0));
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &ALICE), 0);
			assert_eq!(Tokens::total_balance(TEST_TOKEN_ID, &ALICE), 100);
			assert_ok!(Tokens::reserve(TEST_TOKEN_ID, &ALICE, 50));
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &ALICE), 50);
			assert_eq!(Tokens::total_balance(TEST_TOKEN_ID, &ALICE), 100);
		});
}

#[test]
fn unreserve_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &ALICE), 0);
			assert_eq!(Tokens::unreserve(TEST_TOKEN_ID, &ALICE, 0), 0);
			assert_eq!(Tokens::unreserve(TEST_TOKEN_ID, &ALICE, 50), 50);
			assert_ok!(Tokens::reserve(TEST_TOKEN_ID, &ALICE, 30));
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 70);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &ALICE), 30);
			assert_eq!(Tokens::unreserve(TEST_TOKEN_ID, &ALICE, 15), 0);
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 85);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &ALICE), 15);
			assert_eq!(Tokens::unreserve(TEST_TOKEN_ID, &ALICE, 30), 15);
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &ALICE), 0);
		});
}

#[test]
fn slash_reserved_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::reserve(TEST_TOKEN_ID, &ALICE, 50));
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 200);
			assert_eq!(Tokens::slash_reserved(TEST_TOKEN_ID, &ALICE, 0), 0);
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 200);
			assert_eq!(Tokens::slash_reserved(TEST_TOKEN_ID, &ALICE, 100), 50);
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &ALICE), 0);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 150);
		});
}

#[test]
fn repatriate_reserved_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &ALICE), 0);
			assert_eq!(
				Tokens::repatriate_reserved(TEST_TOKEN_ID, &ALICE, &ALICE, 0, BalanceStatus::Free),
				Ok(0)
			);
			assert_eq!(
				Tokens::repatriate_reserved(TEST_TOKEN_ID, &ALICE, &ALICE, 50, BalanceStatus::Free),
				Ok(50)
			);
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &ALICE), 0);

			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &BOB), 100);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &BOB), 0);
			assert_ok!(Tokens::reserve(TEST_TOKEN_ID, &BOB, 50));
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &BOB), 50);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &BOB), 50);
			assert_eq!(
				Tokens::repatriate_reserved(TEST_TOKEN_ID, &BOB, &BOB, 60, BalanceStatus::Reserved),
				Ok(10)
			);
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &BOB), 50);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &BOB), 50);

			assert_eq!(
				Tokens::repatriate_reserved(TEST_TOKEN_ID, &BOB, &ALICE, 30, BalanceStatus::Reserved),
				Ok(0)
			);
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 100);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &ALICE), 30);
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &BOB), 50);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &BOB), 20);

			assert_eq!(
				Tokens::repatriate_reserved(TEST_TOKEN_ID, &BOB, &ALICE, 30, BalanceStatus::Free),
				Ok(10)
			);
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 120);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &ALICE), 30);
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &BOB), 50);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &BOB), 0);
		});
}

#[test]
fn slash_draw_reserved_correct() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::reserve(TEST_TOKEN_ID, &ALICE, 50));
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 50);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 200);

			assert_eq!(Tokens::slash(TEST_TOKEN_ID, &ALICE, 80), 0);
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 0);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &ALICE), 20);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 120);

			assert_eq!(Tokens::slash(TEST_TOKEN_ID, &ALICE, 50), 30);
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 0);
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &ALICE), 0);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 100);
		});
}

#[test]
fn genesis_issuance_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 100);
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &BOB), 100);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 200);
		});
}

#[test]
fn transfer_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::transfer(Some(ALICE).into(), BOB, TEST_TOKEN_ID, 50));
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 50);
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &BOB), 150);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 200);

			let transferred_event = TestEvent::tokens(RawEvent::Transferred(TEST_TOKEN_ID, ALICE, BOB, 50));
			assert!(System::events().iter().any(|record| record.event == transferred_event));

			assert_noop!(
				Tokens::transfer(Some(ALICE).into(), BOB, TEST_TOKEN_ID, 60),
				Error::<Runtime>::BalanceTooLow,
			);
		});
}

#[test]
fn transfer_fails_if_below_existential_deposit() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_noop!(
				Tokens::transfer(Some(ALICE).into(), CHARLIE, TEST_TOKEN_ID, 1),
				Error::<Runtime>::ExistentialDeposit
			);
		});
}

#[test]
fn transfer_enforces_existential_rule() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::transfer(Some(ALICE).into(), BOB, TEST_TOKEN_ID, 99));
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 0);
			assert_eq!(MockDustRemoval::accumulated_dust(), 1);
		});
}

#[test]
fn transfer_all_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::transfer_all(Some(ALICE).into(), BOB, TEST_TOKEN_ID));
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 0);
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &BOB), 200);

			let transferred_event = TestEvent::tokens(RawEvent::Transferred(TEST_TOKEN_ID, ALICE, BOB, 100));
			assert!(System::events().iter().any(|record| record.event == transferred_event));
		});
}

#[test]
fn deposit_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::deposit(TEST_TOKEN_ID, &ALICE, 100));
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 200);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 300);

			assert_noop!(
				Tokens::deposit(TEST_TOKEN_ID, &ALICE, Balance::max_value()),
				Error::<Runtime>::TotalIssuanceOverflow,
			);
		});
}

#[test]
fn deposit_enforces_existential_rule() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::deposit(TEST_TOKEN_ID, &CHARLIE, 1));
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &CHARLIE), 0);
			assert_eq!(MockDustRemoval::accumulated_dust(), 0);
		});
}

#[test]
fn withdraw_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::withdraw(TEST_TOKEN_ID, &ALICE, 50));
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 150);

			assert_noop!(
				Tokens::withdraw(TEST_TOKEN_ID, &ALICE, 60),
				Error::<Runtime>::BalanceTooLow
			);
		});
}

#[test]
fn withdraw_enforces_existential_rule() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::withdraw(TEST_TOKEN_ID, &ALICE, 99));
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 0);
			assert_eq!(MockDustRemoval::accumulated_dust(), 1);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 100);
		});
}

#[test]
fn slash_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			// slashed_amount < amount
			assert_eq!(Tokens::slash(TEST_TOKEN_ID, &ALICE, 50), 0);
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 150);

			// slashed_amount == amount
			assert_eq!(Tokens::slash(TEST_TOKEN_ID, &ALICE, 51), 1);
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 0);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 100);
		});
}

#[test]
fn slash_enforces_existential_rule() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::slash(TEST_TOKEN_ID, &ALICE, 99), 0);
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 0);
			assert_eq!(MockDustRemoval::accumulated_dust(), 1);
		});
}

#[test]
fn update_balance_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::update_balance(TEST_TOKEN_ID, &ALICE, 50));
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 150);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 250);

			assert_ok!(Tokens::update_balance(TEST_TOKEN_ID, &BOB, -50));
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &BOB), 50);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 200);

			assert_noop!(
				Tokens::update_balance(TEST_TOKEN_ID, &BOB, -60),
				Error::<Runtime>::BalanceTooLow
			);
		});
}

#[test]
fn ensure_can_withdraw_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_noop!(
				Tokens::ensure_can_withdraw(TEST_TOKEN_ID, &ALICE, 101),
				Error::<Runtime>::BalanceTooLow
			);

			assert_ok!(Tokens::ensure_can_withdraw(TEST_TOKEN_ID, &ALICE, 1));
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 100);
		});
}

#[test]
fn no_op_if_amount_is_zero() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(Tokens::ensure_can_withdraw(TEST_TOKEN_ID, &ALICE, 0));
		assert_ok!(Tokens::transfer(Some(ALICE).into(), BOB, TEST_TOKEN_ID, 0));
		assert_ok!(Tokens::transfer(Some(ALICE).into(), ALICE, TEST_TOKEN_ID, 0));
		assert_ok!(Tokens::deposit(TEST_TOKEN_ID, &ALICE, 0));
		assert_ok!(Tokens::withdraw(TEST_TOKEN_ID, &ALICE, 0));
		assert_eq!(Tokens::slash(TEST_TOKEN_ID, &ALICE, 0), 0);
		assert_eq!(Tokens::slash(TEST_TOKEN_ID, &ALICE, 1), 1);
		assert_ok!(Tokens::update_balance(TEST_TOKEN_ID, &ALICE, 0));
	});
}
