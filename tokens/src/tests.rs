//! Unit tests for the tokens module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{Balance, ExtBuilder, System, TestEvent, Tokens, ALICE, BOB, TEST_TOKEN_ID};

#[test]
fn genesis_issuance_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::balance(TEST_TOKEN_ID, &ALICE), 100);
			assert_eq!(Tokens::balance(TEST_TOKEN_ID, &BOB), 100);
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
			assert_eq!(Tokens::balance(TEST_TOKEN_ID, &ALICE), 50);
			assert_eq!(Tokens::balance(TEST_TOKEN_ID, &BOB), 150);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 200);

			let transferred_event = TestEvent::tokens(RawEvent::Transferred(TEST_TOKEN_ID, ALICE, BOB, 50));
			assert!(System::events().iter().any(|record| record.event == transferred_event));

			assert_noop!(
				Tokens::transfer(Some(ALICE).into(), BOB, TEST_TOKEN_ID, 60),
				Error::BalanceTooLow.into(),
			);
		});
}

#[test]
fn deposit_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::deposit(TEST_TOKEN_ID, &ALICE, 100));
			assert_eq!(Tokens::balance(TEST_TOKEN_ID, &ALICE), 200);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 300);

			assert_noop!(
				Tokens::deposit(TEST_TOKEN_ID, &ALICE, Balance::max_value()),
				Error::TotalIssuanceOverflow,
			);
		});
}

#[test]
fn withdraw_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::withdraw(TEST_TOKEN_ID, &ALICE, 50));
			assert_eq!(Tokens::balance(TEST_TOKEN_ID, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 150);

			assert_noop!(Tokens::withdraw(TEST_TOKEN_ID, &ALICE, 60), Error::BalanceTooLow);
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
			assert_eq!(Tokens::balance(TEST_TOKEN_ID, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 150);

			// slashed_amount == amount
			assert_eq!(Tokens::slash(TEST_TOKEN_ID, &ALICE, 51), 1);
			assert_eq!(Tokens::balance(TEST_TOKEN_ID, &ALICE), 0);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 100);
		});
}

#[test]
fn update_balance_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Tokens::update_balance(TEST_TOKEN_ID, &ALICE, 50));
			assert_eq!(Tokens::balance(TEST_TOKEN_ID, &ALICE), 150);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 250);

			assert_ok!(Tokens::update_balance(TEST_TOKEN_ID, &BOB, -50));
			assert_eq!(Tokens::balance(TEST_TOKEN_ID, &BOB), 50);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 200);

			assert_noop!(Tokens::update_balance(TEST_TOKEN_ID, &BOB, -60), Error::BalanceTooLow);
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
				Error::BalanceTooLow
			);

			assert_ok!(Tokens::ensure_can_withdraw(TEST_TOKEN_ID, &ALICE, 1));
			assert_eq!(Tokens::balance(TEST_TOKEN_ID, &ALICE), 100);
		});
}
