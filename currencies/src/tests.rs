//! Unit tests for the currencies module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{
	AccountId, AdaptedBasicCurrency, Currencies, ExtBuilder, NativeCurrency, Origin, PalletBalances, System, TestEvent,
	ALICE, BOB, EVA, NATIVE_CURRENCY_ID, X_TOKEN_ID,
};
use sp_runtime::traits::BadOrigin;

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
fn multi_currency_extended_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(<Currencies as MultiCurrencyExtended<AccountId>>::update_balance(
				X_TOKEN_ID, &ALICE, 50
			));
			assert_eq!(Currencies::balance(X_TOKEN_ID, &ALICE), 150);
		});
}

#[test]
fn native_currency_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.make_for_pallet_balances()
		.build()
		.execute_with(|| {
			assert_ok!(Currencies::transfer_native_currency(Some(ALICE).into(), BOB, 50));
			assert_eq!(NativeCurrency::balance(&ALICE), 50);
			assert_eq!(NativeCurrency::balance(&BOB), 150);

			assert_ok!(NativeCurrency::transfer(&ALICE, &BOB, 10));
			assert_eq!(NativeCurrency::balance(&ALICE), 40);
			assert_eq!(NativeCurrency::balance(&BOB), 160);

			assert_eq!(Currencies::slash(NATIVE_CURRENCY_ID, &ALICE, 10), 0);
			assert_eq!(NativeCurrency::balance(&ALICE), 30);
			assert_eq!(NativeCurrency::total_issuance(), 190);
		});
}

#[test]
fn native_currency_extended_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.make_for_pallet_balances()
		.build()
		.execute_with(|| {
			assert_ok!(NativeCurrency::update_balance(&ALICE, 10));
			assert_eq!(NativeCurrency::balance(&ALICE), 110);

			assert_ok!(<Currencies as MultiCurrencyExtended<AccountId>>::update_balance(
				NATIVE_CURRENCY_ID,
				&ALICE,
				10
			));
			assert_eq!(NativeCurrency::balance(&ALICE), 120);
		});
}

#[test]
fn basic_currency_adapting_pallet_balances_transfer() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.make_for_pallet_balances()
		.build()
		.execute_with(|| {
			assert_ok!(AdaptedBasicCurrency::transfer(&ALICE, &BOB, 50));
			assert_eq!(PalletBalances::total_balance(&ALICE), 50);
			assert_eq!(PalletBalances::total_balance(&BOB), 150);

			// creation fee
			assert_ok!(AdaptedBasicCurrency::transfer(&ALICE, &EVA, 10));
			assert_eq!(PalletBalances::total_balance(&ALICE), 40);
			assert_eq!(PalletBalances::total_balance(&EVA), 10);
		});
}

#[test]
fn basic_currency_adapting_pallet_balances_deposit() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.make_for_pallet_balances()
		.build()
		.execute_with(|| {
			assert_ok!(AdaptedBasicCurrency::deposit(&EVA, 50));
			assert_eq!(PalletBalances::total_balance(&EVA), 50);
			assert_eq!(PalletBalances::total_issuance(), 250);
		});
}

#[test]
fn basic_currency_adapting_pallet_balances_withdraw() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.make_for_pallet_balances()
		.build()
		.execute_with(|| {
			assert_ok!(AdaptedBasicCurrency::withdraw(&ALICE, 100));
			assert_eq!(PalletBalances::total_balance(&ALICE), 0);
			assert_eq!(PalletBalances::total_issuance(), 100);
		});
}

#[test]
fn basic_currency_adapting_pallet_balances_slash() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.make_for_pallet_balances()
		.build()
		.execute_with(|| {
			assert_eq!(AdaptedBasicCurrency::slash(&ALICE, 101), 1);
			assert_eq!(PalletBalances::total_balance(&ALICE), 0);
			assert_eq!(PalletBalances::total_issuance(), 100);
		});
}

#[test]
fn basic_currency_adapting_pallet_balances_update_balance() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.make_for_pallet_balances()
		.build()
		.execute_with(|| {
			assert_ok!(AdaptedBasicCurrency::update_balance(&ALICE, -10));
			assert_eq!(PalletBalances::total_balance(&ALICE), 90);
			assert_eq!(PalletBalances::total_issuance(), 190);
		});
}

#[test]
fn update_balance_call_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.make_for_pallet_balances()
		.build()
		.execute_with(|| {
			assert_ok!(Currencies::update_balance(Origin::ROOT, ALICE, NATIVE_CURRENCY_ID, -10));
			assert_eq!(NativeCurrency::balance(&ALICE), 90);

			assert_ok!(Currencies::update_balance(Origin::ROOT, ALICE, X_TOKEN_ID, 10));
			assert_eq!(Currencies::balance(X_TOKEN_ID, &ALICE), 10);
		});
}

#[test]
fn update_balance_call_fails_if_not_root_origin() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(
			Currencies::update_balance(Some(ALICE).into(), ALICE, X_TOKEN_ID, 100),
			BadOrigin
		);
	});
}

#[test]
fn call_event_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			assert_ok!(Currencies::transfer(Some(ALICE).into(), BOB, X_TOKEN_ID, 50));
			assert_eq!(Currencies::balance(X_TOKEN_ID, &ALICE), 50);
			assert_eq!(Currencies::balance(X_TOKEN_ID, &BOB), 150);

			let transferred_event = TestEvent::currencies(RawEvent::Transferred(X_TOKEN_ID, ALICE, BOB, 50));
			assert!(System::events().iter().any(|record| record.event == transferred_event));

			assert_ok!(<Currencies as MultiCurrency<AccountId>>::transfer(
				X_TOKEN_ID, &ALICE, &BOB, 10
			));
			assert_eq!(Currencies::balance(X_TOKEN_ID, &ALICE), 40);
			assert_eq!(Currencies::balance(X_TOKEN_ID, &BOB), 160);

			let transferred_event = TestEvent::currencies(RawEvent::Transferred(X_TOKEN_ID, ALICE, BOB, 10));
			assert!(System::events().iter().any(|record| record.event == transferred_event));

			assert_ok!(<Currencies as MultiCurrency<AccountId>>::deposit(
				X_TOKEN_ID, &ALICE, 100
			));
			assert_eq!(Currencies::balance(X_TOKEN_ID, &ALICE), 140);

			let transferred_event = TestEvent::currencies(RawEvent::Deposited(X_TOKEN_ID, ALICE, 100));
			assert!(System::events().iter().any(|record| record.event == transferred_event));

			assert_ok!(<Currencies as MultiCurrency<AccountId>>::withdraw(
				X_TOKEN_ID, &ALICE, 20
			));
			assert_eq!(Currencies::balance(X_TOKEN_ID, &ALICE), 120);

			let transferred_event = TestEvent::currencies(RawEvent::Withdrawn(X_TOKEN_ID, ALICE, 20));
			assert!(System::events().iter().any(|record| record.event == transferred_event));
		});
}
