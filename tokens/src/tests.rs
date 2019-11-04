//! Unit tests for the tokens module.

#![cfg(test)]

use super::*;
use mock::{Balance, ExtBuilder, Runtime, Tokens, ALICE, BOB, TEST_TOKEN_ID};
use srml_support::{assert_noop, assert_ok};

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
fn positive_imbalance_increases_total_issuance() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			let positive = <PositiveImbalance<Runtime>>::new(TEST_TOKEN_ID, 100);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 200);

			drop(positive);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 300);
		});
}

#[test]
fn negative_imbalance_reduces_total_issuance() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			let negative = <NegativeImbalance<Runtime>>::new(TEST_TOKEN_ID, 100);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 200);

			drop(negative);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 100);
		});
}

#[test]
fn rebalance_when_overflow_should_saturate() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			let positive = <PositiveImbalance<Runtime>>::new(TEST_TOKEN_ID, Balance::max_value());
			drop(positive);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), Balance::max_value());
		});
}

#[test]
fn rebalance_when_underflow_should_saturate() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			let negative = <NegativeImbalance<Runtime>>::new(TEST_TOKEN_ID, 300);
			drop(negative);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 0);
		});
}

#[test]
fn transfer_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(<Tokens as MultiCurrency<_>>::transfer(TEST_TOKEN_ID, &ALICE, &BOB, 50));
			assert_eq!(Tokens::balance(TEST_TOKEN_ID, &ALICE), 50);
			assert_eq!(Tokens::balance(TEST_TOKEN_ID, &BOB), 150);

			assert_noop!(
				<Tokens as MultiCurrency<_>>::transfer(TEST_TOKEN_ID, &ALICE, &BOB, 60),
				"balance too low",
			);
			assert_eq!(Tokens::balance(TEST_TOKEN_ID, &ALICE), 50);
			assert_eq!(Tokens::balance(TEST_TOKEN_ID, &BOB), 150);
		});
}

#[test]
fn deposit_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			let positive = Tokens::deposit(TEST_TOKEN_ID, &ALICE, 100);
			assert_eq!(Tokens::balance(TEST_TOKEN_ID, &ALICE), 200);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 200);

			drop(positive);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 300);

			assert_noop!(
				Tokens::deposit(TEST_TOKEN_ID, &ALICE, Balance::max_value()),
				"total issuance overflow after deposit",
			);
		});
}

#[test]
fn withdraw_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			let positive = Tokens::withdraw(TEST_TOKEN_ID, &ALICE, 50);
			assert_eq!(Tokens::balance(TEST_TOKEN_ID, &ALICE), 50);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 200);

			drop(positive);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 150);

			assert_noop!(
				Tokens::withdraw(TEST_TOKEN_ID, &ALICE, 60),
				"balance too low to withdraw",
			);
		});
}
