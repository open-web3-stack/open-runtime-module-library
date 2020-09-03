//! Unit tests for the tokens module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok, traits::WithdrawReason};
use mock::{
	Balance, ExtBuilder, Runtime, System, TestEvent, Tokens, TreasuryCurrencyAdapter, ACCUMULATED_RECEIVED, ALICE, BOB,
	ID_1, ID_2, TEST_TOKEN_ID, TREASURY_ACCOUNT,
};

#[test]
fn set_lock_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			Tokens::set_lock(ID_1, TEST_TOKEN_ID, &ALICE, 10);
			assert_eq!(Tokens::accounts(&ALICE, TEST_TOKEN_ID).frozen, 10);
			assert_eq!(Tokens::accounts(&ALICE, TEST_TOKEN_ID).frozen(), 10);
			assert_eq!(Tokens::locks(ALICE, TEST_TOKEN_ID).len(), 1);
			Tokens::set_lock(ID_1, TEST_TOKEN_ID, &ALICE, 50);
			assert_eq!(Tokens::accounts(&ALICE, TEST_TOKEN_ID).frozen, 50);
			assert_eq!(Tokens::locks(ALICE, TEST_TOKEN_ID).len(), 1);
			Tokens::set_lock(ID_2, TEST_TOKEN_ID, &ALICE, 60);
			assert_eq!(Tokens::accounts(&ALICE, TEST_TOKEN_ID).frozen, 60);
			assert_eq!(Tokens::locks(ALICE, TEST_TOKEN_ID).len(), 2);
		});
}

#[test]
fn extend_lock_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			Tokens::set_lock(ID_1, TEST_TOKEN_ID, &ALICE, 10);
			assert_eq!(Tokens::locks(ALICE, TEST_TOKEN_ID).len(), 1);
			assert_eq!(Tokens::accounts(&ALICE, TEST_TOKEN_ID).frozen, 10);
			Tokens::extend_lock(ID_1, TEST_TOKEN_ID, &ALICE, 20);
			assert_eq!(Tokens::locks(ALICE, TEST_TOKEN_ID).len(), 1);
			assert_eq!(Tokens::accounts(&ALICE, TEST_TOKEN_ID).frozen, 20);
			Tokens::extend_lock(ID_2, TEST_TOKEN_ID, &ALICE, 10);
			Tokens::extend_lock(ID_1, TEST_TOKEN_ID, &ALICE, 20);
			assert_eq!(Tokens::locks(ALICE, TEST_TOKEN_ID).len(), 2);
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
			assert_eq!(Tokens::locks(ALICE, TEST_TOKEN_ID).len(), 2);
			Tokens::remove_lock(ID_2, TEST_TOKEN_ID, &ALICE);
			assert_eq!(Tokens::locks(ALICE, TEST_TOKEN_ID).len(), 1);
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
			System::set_block_number(1);

			assert_ok!(Tokens::transfer(Some(ALICE).into(), BOB, TEST_TOKEN_ID, 50));
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &ALICE), 50);
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &BOB), 150);
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 200);
			assert_eq!(
				ACCUMULATED_RECEIVED.with(|v| *v.borrow().get(&(BOB, TEST_TOKEN_ID)).unwrap()),
				50
			);

			let transferred_event = TestEvent::tokens(RawEvent::Transferred(TEST_TOKEN_ID, ALICE, BOB, 50));
			assert!(System::events().iter().any(|record| record.event == transferred_event));

			assert_noop!(
				Tokens::transfer(Some(ALICE).into(), BOB, TEST_TOKEN_ID, 60),
				Error::<Runtime>::BalanceTooLow,
			);
		});
}

#[test]
fn transfer_all_should_work() {
	ExtBuilder::default()
		.one_hundred_for_alice_n_bob()
		.build()
		.execute_with(|| {
			System::set_block_number(1);

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

#[test]
fn currency_adapter_ensure_currency_adapter_should_work() {
	ExtBuilder::default()
		.one_hundred_for_treasury_account()
		.build()
		.execute_with(|| {
			assert_eq!(Tokens::total_issuance(TEST_TOKEN_ID), 100);
			assert_eq!(Tokens::total_balance(TEST_TOKEN_ID, &TREASURY_ACCOUNT), 100);
			// CandidacyBond = 3 VotingBond = 2
			assert_eq!(Tokens::reserved_balance(TEST_TOKEN_ID, &TREASURY_ACCOUNT), 5);
			assert_eq!(Tokens::free_balance(TEST_TOKEN_ID, &TREASURY_ACCOUNT), 95);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::total_balance(&TREASURY_ACCOUNT),
				100
			);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::can_slash(&TREASURY_ACCOUNT, 10),
				true
			);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::total_issuance(),
				100
			);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::minimum_balance(),
				0
			);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::can_reserve(&TREASURY_ACCOUNT, 5),
				true
			);

			// burn
			let imbalance = <Runtime as pallet_elections_phragmen::Trait>::Currency::burn(10);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::total_issuance(),
				90
			);
			drop(imbalance);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::total_issuance(),
				100
			);

			// issue
			let imbalance = <Runtime as pallet_elections_phragmen::Trait>::Currency::issue(20);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::total_issuance(),
				120
			);
			drop(imbalance);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::total_issuance(),
				100
			);

			// transfer
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::free_balance(&TREASURY_ACCOUNT),
				95
			);
			assert_ok!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::ensure_can_withdraw(
					&TREASURY_ACCOUNT,
					10,
					WithdrawReason::Transfer.into(),
					0
				)
			);
			assert_ok!(<Runtime as pallet_elections_phragmen::Trait>::Currency::transfer(
				&TREASURY_ACCOUNT,
				&ALICE,
				11,
				ExistenceRequirement::KeepAlive
			));
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::free_balance(&TREASURY_ACCOUNT),
				84
			);

			// deposit
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::total_issuance(),
				100
			);
			let imbalance = TreasuryCurrencyAdapter::deposit_creating(&TREASURY_ACCOUNT, 11);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::free_balance(&TREASURY_ACCOUNT),
				95
			);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::total_issuance(),
				100
			);
			drop(imbalance);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::free_balance(&TREASURY_ACCOUNT),
				95
			);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::total_issuance(),
				111
			);

			// withdraw
			let imbalance = <Runtime as pallet_elections_phragmen::Trait>::Currency::withdraw(
				&TREASURY_ACCOUNT,
				10,
				WithdrawReason::Transfer.into(),
				ExistenceRequirement::KeepAlive,
			);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::free_balance(&TREASURY_ACCOUNT),
				85
			);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::total_issuance(),
				111
			);
			drop(imbalance);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::free_balance(&TREASURY_ACCOUNT),
				85
			);
			assert_eq!(
				<Runtime as pallet_elections_phragmen::Trait>::Currency::total_issuance(),
				101
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
		Tokens::set_reserved_balance(TEST_TOKEN_ID, &TREASURY_ACCOUNT, 69);
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
		assert_eq!(TreasuryCurrencyAdapter::total_issuance(), 42);
	});
}

#[test]
fn currency_adapter_slashing_incomplete_balance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let _ = TreasuryCurrencyAdapter::deposit_creating(&TREASURY_ACCOUNT, 42);
		assert_ok!(TreasuryCurrencyAdapter::reserve(&TREASURY_ACCOUNT, 21));
		assert_eq!(TreasuryCurrencyAdapter::slash(&TREASURY_ACCOUNT, 69).1, 27);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT), 0);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&TREASURY_ACCOUNT), 0);
		assert_eq!(TreasuryCurrencyAdapter::total_issuance(), 0);
	});
}

#[test]
fn currency_adapter_basic_locking_should_work() {
	ExtBuilder::default()
		.one_hundred_for_treasury_account()
		.build()
		.execute_with(|| {
			// CandidacyBond = 3 VotingBond = 2
			assert_eq!(TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT), 95);
			TreasuryCurrencyAdapter::set_lock(ID_1, &TREASURY_ACCOUNT, 91, WithdrawReasons::all());
			assert_noop!(
				TreasuryCurrencyAdapter::transfer(&TREASURY_ACCOUNT, &ALICE, 5, ExistenceRequirement::AllowDeath),
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
				1,
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
				1,
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
				1,
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
			TreasuryCurrencyAdapter::set_lock(ID_1, &TREASURY_ACCOUNT, 5, WithdrawReasons::none());
			TreasuryCurrencyAdapter::set_lock(ID_2, &TREASURY_ACCOUNT, 5, WithdrawReasons::all());
			assert_ok!(TreasuryCurrencyAdapter::transfer(
				&TREASURY_ACCOUNT,
				&ALICE,
				1,
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
			TreasuryCurrencyAdapter::set_lock(ID_1, &TREASURY_ACCOUNT, u64::max_value(), WithdrawReasons::none());
			TreasuryCurrencyAdapter::set_lock(ID_2, &TREASURY_ACCOUNT, 0, WithdrawReasons::all());
			assert_noop!(
				TreasuryCurrencyAdapter::transfer(&TREASURY_ACCOUNT, &ALICE, 1, ExistenceRequirement::AllowDeath),
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
			TreasuryCurrencyAdapter::set_lock(ID_1, &TREASURY_ACCOUNT, 90, WithdrawReason::Transfer.into());
			assert_noop!(
				TreasuryCurrencyAdapter::transfer(&TREASURY_ACCOUNT, &ALICE, 6, ExistenceRequirement::AllowDeath),
				Error::<Runtime>::LiquidityRestrictions
			);
			TreasuryCurrencyAdapter::extend_lock(ID_1, &TREASURY_ACCOUNT, 90, WithdrawReasons::none());
			assert_noop!(
				TreasuryCurrencyAdapter::transfer(&TREASURY_ACCOUNT, &ALICE, 6, ExistenceRequirement::AllowDeath),
				Error::<Runtime>::LiquidityRestrictions
			);
			TreasuryCurrencyAdapter::extend_lock(ID_1, &TREASURY_ACCOUNT, 90, WithdrawReason::Reserve.into());
			assert_noop!(
				TreasuryCurrencyAdapter::transfer(&TREASURY_ACCOUNT, &ALICE, 6, ExistenceRequirement::AllowDeath),
				Error::<Runtime>::LiquidityRestrictions
			);
		});
}

#[test]
fn currency_adapter_reward_should_work() {
	ExtBuilder::default()
		.one_hundred_for_treasury_account()
		.build()
		.execute_with(|| {
			assert_eq!(TreasuryCurrencyAdapter::total_issuance(), 100);
			assert_eq!(TreasuryCurrencyAdapter::total_balance(&TREASURY_ACCOUNT), 100);
			assert_ok!(TreasuryCurrencyAdapter::deposit_into_existing(&TREASURY_ACCOUNT, 10).map(drop));
			assert_eq!(TreasuryCurrencyAdapter::total_balance(&TREASURY_ACCOUNT), 110);
			assert_eq!(TreasuryCurrencyAdapter::total_issuance(), 110);
		});
}

#[test]
fn currency_adapter_slashing_reserved_balance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let _ = TreasuryCurrencyAdapter::deposit_creating(&TREASURY_ACCOUNT, 111);
		assert_ok!(TreasuryCurrencyAdapter::reserve(&TREASURY_ACCOUNT, 111));
		assert_eq!(TreasuryCurrencyAdapter::slash_reserved(&TREASURY_ACCOUNT, 42).1, 0);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&TREASURY_ACCOUNT), 69);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT), 0);
		assert_eq!(TreasuryCurrencyAdapter::total_issuance(), 69);
	});
}

#[test]
fn currency_adapter_slashing_incomplete_reserved_balance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let _ = TreasuryCurrencyAdapter::deposit_creating(&TREASURY_ACCOUNT, 111);
		assert_ok!(TreasuryCurrencyAdapter::reserve(&TREASURY_ACCOUNT, 42));
		assert_eq!(TreasuryCurrencyAdapter::slash_reserved(&TREASURY_ACCOUNT, 69).1, 27);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT), 69);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&TREASURY_ACCOUNT), 0);
		assert_eq!(TreasuryCurrencyAdapter::total_issuance(), 69);
	});
}

#[test]
fn currency_adapter_repatriating_reserved_balance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let _ = TreasuryCurrencyAdapter::deposit_creating(&TREASURY_ACCOUNT, 110);
		let _ = TreasuryCurrencyAdapter::deposit_creating(&ALICE, 1);
		assert_ok!(TreasuryCurrencyAdapter::reserve(&TREASURY_ACCOUNT, 110));
		assert_ok!(
			TreasuryCurrencyAdapter::repatriate_reserved(&TREASURY_ACCOUNT, &ALICE, 41, Status::Free),
			0
		);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&TREASURY_ACCOUNT), 69);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT), 0);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&ALICE), 0);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&ALICE), 42);
	});
}

#[test]
fn currency_adapter_transferring_reserved_balance_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let _ = TreasuryCurrencyAdapter::deposit_creating(&TREASURY_ACCOUNT, 110);
		let _ = TreasuryCurrencyAdapter::deposit_creating(&ALICE, 1);
		assert_ok!(TreasuryCurrencyAdapter::reserve(&TREASURY_ACCOUNT, 110));
		assert_ok!(
			TreasuryCurrencyAdapter::repatriate_reserved(&TREASURY_ACCOUNT, &ALICE, 41, Status::Reserved),
			0
		);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&TREASURY_ACCOUNT), 69);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT), 0);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&ALICE), 41);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&ALICE), 1);
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
		let _ = TreasuryCurrencyAdapter::deposit_creating(&ALICE, 1);
		assert_ok!(TreasuryCurrencyAdapter::reserve(&TREASURY_ACCOUNT, 41));
		assert_ok!(
			TreasuryCurrencyAdapter::repatriate_reserved(&TREASURY_ACCOUNT, &ALICE, 69, Status::Free),
			28
		);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&TREASURY_ACCOUNT), 0);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT), 69);
		assert_eq!(TreasuryCurrencyAdapter::reserved_balance(&ALICE), 0);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&ALICE), 42);
	});
}

#[test]
fn currency_adapter_transferring_too_high_value_should_not_panic() {
	ExtBuilder::default().build().execute_with(|| {
		TreasuryCurrencyAdapter::make_free_balance_be(&TREASURY_ACCOUNT, u64::max_value());
		TreasuryCurrencyAdapter::make_free_balance_be(&ALICE, 1);

		assert_noop!(
			TreasuryCurrencyAdapter::transfer(
				&TREASURY_ACCOUNT,
				&ALICE,
				u64::max_value(),
				ExistenceRequirement::AllowDeath
			),
			Error::<Runtime>::BalanceOverflow,
		);

		assert_eq!(
			TreasuryCurrencyAdapter::free_balance(&TREASURY_ACCOUNT),
			u64::max_value()
		);
		assert_eq!(TreasuryCurrencyAdapter::free_balance(&ALICE), 1);
	});
}
