//! Unit tests for the tokens module.

#![cfg(test)]

use super::*;
use mock::{Tokens, ExtBuilder, ALICE, BOB, TEST_TOKEN_ID};
use srml_support::{assert_noop, assert_ok, assert_err};
use system::RawOrigin;

#[test]
fn genesis_issuance_should_work() {
	ExtBuilder::default().one_hundred_for_alice_n_bob().build().execute_with(|| {
		assert_eq!(Tokens::balance(TEST_TOKEN_ID, &ALICE), 100);
		assert_eq!(Tokens::balance(TEST_TOKEN_ID, &BOB), 100);
	});
}
