//! Unit tests for the tokens module.

#![cfg(test)]

use super::*;
use mock::{Balance, ExtBuilder, Runtime, Tokens, ALICE, BOB, TEST_TOKEN_ID};
use srml_support::{assert_err, assert_noop, assert_ok};
use system::RawOrigin;

#[test]
fn genesis_issuance_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::balance(TEST_TOKEN_ID, &ALICE), 100);
			assert_eq!(Tokens::balance(TEST_TOKEN_ID, &BOB), 100);
		});
}

#[test]
fn positive_imbalance_increases_total_issuance() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			{
				let _ = <PositiveImbalance<Runtime>>::new(TEST_TOKEN_ID, 100);
			}
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 300);
		});
}

#[test]
fn negative_imbalance_reduces_total_issuance() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			{
				let _ = <NegativeImbalance<Runtime>>::new(TEST_TOKEN_ID, 100);
			}
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 100);
		});
}

#[test]
fn rebalance_when_overflow_should_saturate() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 200);
			{
				let _ = <PositiveImbalance<Runtime>>::new(TEST_TOKEN_ID, Balance::max_value());
			}
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), Balance::max_value());
		});
}

#[test]
fn rebalance_when_underflow_should_saturate() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 200);
			{
				let _ = <NegativeImbalance<Runtime>>::new(TEST_TOKEN_ID, 300);
			}
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 0);
		});
}
