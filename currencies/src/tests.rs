//! Unit tests for the tokens module.

#![cfg(test)]

use super::*;
use mock::{Currencies, ExtBuilder, NativeCurrency, ALICE, BOB, X_TOKEN_ID};
use srml_support::assert_ok;

#[test]
fn multi_currency_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Currencies::transfer(Some(ALICE).into(), BOB, X_TOKEN_ID, 50));
			assert_eq!(Currencies::balance(X_TOKEN_ID, &ALICE), 50);
			assert_eq!(Currencies::balance(X_TOKEN_ID, &BOB), 150);
		});
}

#[test]
fn native_currency_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Currencies::transfer_native_currency(Some(ALICE).into(), BOB, 50));
			assert_eq!(NativeCurrency::balance(&ALICE), 50);
			assert_eq!(NativeCurrency::balance(&BOB), 150);

			assert_ok!(NativeCurrency::transfer(&ALICE, &BOB, 10));
			assert_eq!(NativeCurrency::balance(&ALICE), 40);
			assert_eq!(NativeCurrency::balance(&BOB), 160);
		});
}
