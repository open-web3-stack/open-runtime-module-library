//! Unit tests for the tokens module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;
use sp_runtime::{ArithmeticError, TokenError};

const REASON: &() = &();

#[test]
fn fungibles_inspect_trait_should_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(<Tokens as fungibles::Inspect<_>>::total_issuance(DOT), 100);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::minimum_balance(DOT), 2);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 100);
			assert_eq!(
				<Tokens as fungibles::Inspect<_>>::reducible_balance(
					DOT,
					&ALICE,
					Preservation::Protect,
					Fortitude::Polite
				),
				98
			);
			assert_eq!(
				<Tokens as fungibles::Inspect<_>>::reducible_balance(
					DOT,
					&ALICE,
					Preservation::Preserve,
					Fortitude::Polite
				),
				98
			);
			assert_ok!(
				<Tokens as fungibles::Inspect<_>>::can_deposit(DOT, &ALICE, 1, Provenance::Extant).into_result()
			);
			assert_ok!(<Tokens as fungibles::Inspect<_>>::can_withdraw(DOT, &ALICE, 1).into_result(true));

			assert!(<Tokens as fungibles::Inspect<_>>::asset_exists(DOT));
			assert!(!<Tokens as fungibles::Inspect<_>>::asset_exists(BTC));
		});
}

#[test]
fn fungibles_mutate_trait_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(<Tokens as fungibles::Mutate<_>>::mint_into(DOT, &ALICE, 10));
		assert_eq!(
			<Tokens as fungibles::Mutate<_>>::burn_from(
				DOT,
				&ALICE,
				8,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Polite
			),
			Ok(8)
		);
	});
}

#[test]
fn fungibles_transfer_trait_should_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 100);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &BOB), 100);
			assert_ok!(<Tokens as fungibles::Mutate<_>>::transfer(
				DOT,
				&ALICE,
				&BOB,
				10,
				Preservation::Protect
			));
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 90);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &BOB), 110);
		});
}

#[test]
fn fungibles_unbalanced_trait_should_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			// set_balance
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 100);
			assert_ok!(<Tokens as fungibles::Unbalanced<_>>::write_balance(DOT, &ALICE, 10));
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 10);

			// set_total_issuance
			assert_eq!(<Tokens as fungibles::Inspect<_>>::total_issuance(DOT), 100);
			<Tokens as fungibles::Unbalanced<_>>::set_total_issuance(DOT, 10);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::total_issuance(DOT), 10);

			// decrease_balance
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 10);
			assert_noop!(
				<Tokens as fungibles::Unbalanced<_>>::decrease_balance(
					DOT,
					&ALICE,
					20,
					Precision::Exact,
					Preservation::Protect,
					Fortitude::Polite
				),
				TokenError::FundsUnavailable
			);
			assert_eq!(
				<Tokens as fungibles::Unbalanced<_>>::decrease_balance(
					DOT,
					&ALICE,
					5,
					Precision::Exact,
					Preservation::Protect,
					Fortitude::Polite
				),
				Ok(5)
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 5);
			// new balance < ExistentialDeposits, clean dust
			assert_eq!(
				<Tokens as fungibles::Unbalanced<_>>::decrease_balance(
					DOT,
					&ALICE,
					4,
					Precision::Exact,
					Preservation::Expendable,
					Fortitude::Polite
				),
				Ok(4)
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 0);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::total_balance(DOT, &ALICE), 0);
			// set reserved
			assert_ok!(<Tokens as fungibles::Unbalanced<_>>::write_balance(DOT, &ALICE, 100));
			assert_ok!(<Tokens as MultiReservableCurrency<AccountId>>::reserve(DOT, &ALICE, 50));
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 50);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::total_balance(DOT, &ALICE), 100);
			assert_eq!(
				<Tokens as fungibles::Inspect<_>>::reducible_balance(
					DOT,
					&ALICE,
					Preservation::Protect,
					Fortitude::Polite
				),
				50
			);
			assert_noop!(
				<Tokens as fungibles::Unbalanced<_>>::decrease_balance(
					DOT,
					&ALICE,
					60,
					Precision::Exact,
					Preservation::Protect,
					Fortitude::Polite
				),
				TokenError::FundsUnavailable
			);
			assert_eq!(
				<Tokens as fungibles::Unbalanced<_>>::decrease_balance(
					DOT,
					&ALICE,
					50,
					Precision::Exact,
					Preservation::Protect,
					Fortitude::Polite
				),
				Ok(50)
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 0);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::total_balance(DOT, &ALICE), 50);
			assert_eq!(
				<Tokens as MultiReservableCurrency<AccountId>>::unreserve(DOT, &ALICE, 50),
				0
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 50);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::total_balance(DOT, &ALICE), 50);

			// decrease_balance_at_most
			assert_ok!(<Tokens as fungibles::Unbalanced<_>>::write_balance(DOT, &ALICE, 10));
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 10);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::total_balance(DOT, &ALICE), 10);
			assert_eq!(
				<Tokens as fungibles::Unbalanced<_>>::decrease_balance(
					DOT,
					&ALICE,
					20,
					Precision::BestEffort,
					Preservation::Expendable,
					Fortitude::Polite
				),
				Ok(10)
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 0);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::total_balance(DOT, &ALICE), 0);
			assert_ok!(<Tokens as fungibles::Unbalanced<_>>::write_balance(DOT, &ALICE, 10));
			assert_eq!(
				<Tokens as fungibles::Unbalanced<_>>::decrease_balance(
					DOT,
					&ALICE,
					5,
					Precision::BestEffort,
					Preservation::Protect,
					Fortitude::Polite
				),
				Ok(5)
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 5);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::total_balance(DOT, &ALICE), 5);

			// new balance < ExistentialDeposits, clean dust
			assert_eq!(
				<Tokens as fungibles::Unbalanced<_>>::decrease_balance(
					DOT,
					&ALICE,
					4,
					Precision::BestEffort,
					Preservation::Expendable,
					Fortitude::Polite
				),
				Ok(4)
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 0);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::total_balance(DOT, &ALICE), 0);
			// set reserved
			assert_ok!(<Tokens as fungibles::Unbalanced<_>>::write_balance(DOT, &ALICE, 100));
			assert_ok!(<Tokens as MultiReservableCurrency<AccountId>>::reserve(DOT, &ALICE, 50));
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 50);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::total_balance(DOT, &ALICE), 100);
			assert_eq!(
				<Tokens as fungibles::Inspect<_>>::reducible_balance(
					DOT,
					&ALICE,
					Preservation::Protect,
					Fortitude::Polite
				),
				50
			);
			assert_eq!(
				<Tokens as fungibles::Unbalanced<_>>::decrease_balance(
					DOT,
					&ALICE,
					60,
					Precision::BestEffort,
					Preservation::Protect,
					Fortitude::Polite
				),
				Ok(50),
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 0);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::total_balance(DOT, &ALICE), 50);
			assert_eq!(
				<Tokens as MultiReservableCurrency<AccountId>>::unreserve(DOT, &ALICE, 50),
				0
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 50);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::total_balance(DOT, &ALICE), 50);

			// increase_balance
			assert_ok!(<Tokens as fungibles::Unbalanced<_>>::write_balance(DOT, &ALICE, 0));
			assert_noop!(
				<Tokens as fungibles::Unbalanced<_>>::increase_balance(DOT, &ALICE, 1, Precision::Exact),
				TokenError::BelowMinimum
			);
			assert_eq!(
				<Tokens as fungibles::Unbalanced<_>>::increase_balance(DOT, &ALICE, 2, Precision::Exact),
				Ok(2)
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 2);
			assert_noop!(
				<Tokens as fungibles::Unbalanced<_>>::increase_balance(DOT, &ALICE, Balance::MAX, Precision::Exact),
				ArithmeticError::Overflow
			);

			// increase_balance_at_most
			assert_ok!(<Tokens as fungibles::Unbalanced<_>>::write_balance(DOT, &ALICE, 0));
			assert_eq!(
				<Tokens as fungibles::Unbalanced<_>>::increase_balance(DOT, &ALICE, 1, Precision::BestEffort),
				Ok(0)
			);
			assert_eq!(
				<Tokens as fungibles::Unbalanced<_>>::increase_balance(DOT, &ALICE, 2, Precision::BestEffort),
				Ok(2)
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 2);
			assert_eq!(
				<Tokens as fungibles::Unbalanced<_>>::increase_balance(
					DOT,
					&ALICE,
					Balance::MAX,
					Precision::BestEffort
				),
				Ok(Balance::MAX - 2)
			);
		});
}

#[test]
fn fungibles_balanced_deposit_works() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			let amount = 42;
			let alice_old_balance = <Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE);
			let debt = <Tokens as fungibles::Balanced<_>>::deposit(DOT, &ALICE, amount, Precision::Exact).unwrap();
			assert_eq!(debt.asset(), DOT);
			assert_eq!(debt.peek(), amount);
			let alice_new_balance = <Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE);
			assert_eq!(alice_old_balance + amount, alice_new_balance);

			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Deposited {
				currency_id: DOT,
				who: ALICE,
				amount,
			}));
		});
}

#[test]
fn fungibles_balanced_withdraw_works() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			let amount = 42;
			let alice_old_balance = <Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE);
			let credit = <Tokens as fungibles::Balanced<_>>::withdraw(
				DOT,
				&ALICE,
				amount,
				Precision::Exact,
				Preservation::Protect,
				Fortitude::Polite,
			)
			.unwrap();
			assert_eq!(credit.asset(), DOT);
			assert_eq!(credit.peek(), amount);
			let alice_new_balance = <Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE);
			assert_eq!(alice_old_balance - amount, alice_new_balance);

			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Withdrawn {
				currency_id: DOT,
				who: ALICE,
				amount,
			}));
		});
}

#[test]
fn fungibles_balanced_issue_works() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			let amount = 42;

			let old_total_issuance = <Tokens as fungibles::Inspect<_>>::total_issuance(DOT);
			let credit = <Tokens as fungibles::Balanced<_>>::issue(DOT, amount);
			assert_eq!(credit.asset(), DOT);
			assert_eq!(credit.peek(), amount);
			let new_total_issuance = <Tokens as fungibles::Inspect<_>>::total_issuance(DOT);
			assert_eq!(old_total_issuance + amount, new_total_issuance);

			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Issued {
				currency_id: DOT,
				amount,
			}));
		});
}

#[test]
fn fungibles_balanced_rescind_works() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			let amount = 42;

			let old_total_issuance = <Tokens as fungibles::Inspect<_>>::total_issuance(DOT);
			let debt = <Tokens as fungibles::Balanced<_>>::rescind(DOT, amount);
			assert_eq!(debt.asset(), DOT);
			assert_eq!(debt.peek(), amount);
			let new_total_issuance = <Tokens as fungibles::Inspect<_>>::total_issuance(DOT);
			assert_eq!(old_total_issuance - amount, new_total_issuance);

			System::assert_last_event(RuntimeEvent::Tokens(crate::Event::Rescinded {
				currency_id: DOT,
				amount,
			}));
		});
}

#[test]
fn fungibles_inspect_hold_trait_should_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(
				<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, REASON, &ALICE),
				0
			);
			assert!(<Tokens as fungibles::InspectHold<_>>::can_hold(DOT, REASON, &ALICE, 50));
			assert!(!<Tokens as fungibles::InspectHold<_>>::can_hold(
				DOT, REASON, &ALICE, 100
			));
		});
}

#[test]
fn fungibles_mutate_hold_trait_should_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_noop!(
				<Tokens as fungibles::MutateHold<_>>::hold(DOT, REASON, &ALICE, 200),
				Error::<Runtime>::BalanceTooLow
			);
			assert_eq!(
				<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, REASON, &ALICE),
				0
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 100);

			assert_ok!(<Tokens as fungibles::MutateHold<_>>::hold(DOT, REASON, &ALICE, 100));
			assert_eq!(
				<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, REASON, &ALICE),
				100
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 0);

			assert_eq!(
				<Tokens as fungibles::MutateHold<_>>::release(DOT, REASON, &ALICE, 40, Precision::Exact),
				Ok(40)
			);
			assert_eq!(
				<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, REASON, &ALICE),
				60
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 40);

			// exceed hold amount when not in best_effort
			assert_noop!(
				<Tokens as fungibles::MutateHold<_>>::release(DOT, REASON, &ALICE, 61, Precision::Exact),
				Error::<Runtime>::BalanceTooLow
			);

			// exceed hold amount when in best_effort
			assert_eq!(
				<Tokens as fungibles::MutateHold<_>>::release(DOT, REASON, &ALICE, 61, Precision::BestEffort),
				Ok(60)
			);
			assert_eq!(
				<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, REASON, &ALICE),
				0
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 100);

			assert_ok!(<Tokens as fungibles::MutateHold<_>>::hold(DOT, REASON, &ALICE, 70));
			assert_eq!(
				<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, REASON, &ALICE),
				70
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 30);

			assert_eq!(
				<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, REASON, &BOB),
				0
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &BOB), 100);
			assert_eq!(
				<Tokens as fungibles::MutateHold<_>>::transfer_on_hold(
					DOT,
					REASON,
					&ALICE,
					&BOB,
					5,
					Precision::Exact,
					Restriction::Free,
					Fortitude::Polite
				),
				Ok(5)
			);
			assert_eq!(
				<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, REASON, &ALICE),
				65
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 30);
			assert_eq!(
				<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, REASON, &BOB),
				0
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &BOB), 105);

			assert_eq!(
				<Tokens as fungibles::MutateHold<_>>::transfer_on_hold(
					DOT,
					REASON,
					&ALICE,
					&BOB,
					5,
					Precision::Exact,
					Restriction::OnHold,
					Fortitude::Polite
				),
				Ok(5)
			);
			assert_eq!(
				<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, REASON, &ALICE),
				60
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 30);
			assert_eq!(
				<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, REASON, &BOB),
				5
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &BOB), 105);

			// exceed hold amount when not in best_effort
			assert_noop!(
				<Tokens as fungibles::MutateHold<_>>::transfer_on_hold(
					DOT,
					REASON,
					&ALICE,
					&BOB,
					61,
					Precision::Exact,
					Restriction::OnHold,
					Fortitude::Polite
				),
				Error::<Runtime>::BalanceTooLow
			);

			// exceed hold amount when in best_effort
			assert_eq!(
				<Tokens as fungibles::MutateHold<_>>::transfer_on_hold(
					DOT,
					REASON,
					&ALICE,
					&BOB,
					61,
					Precision::BestEffort,
					Restriction::OnHold,
					Fortitude::Polite
				),
				Ok(60)
			);
			assert_eq!(
				<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, REASON, &ALICE),
				0
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 30);
			assert_eq!(
				<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, REASON, &BOB),
				65
			);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &BOB), 105);
		});
}

#[test]
fn fungibles_inspect_convert_should_work() {
	pub struct ConvertBalanceTest;
	impl ConvertBalance<Balance, Balance> for ConvertBalanceTest {
		type AssetId = CurrencyId;
		fn convert_balance(balance: Balance, _asset_id: CurrencyId) -> Result<Balance, ArithmeticError> {
			Ok(balance * 100)
		}

		fn convert_balance_back(balance: Balance, _asset_id: CurrencyId) -> Result<Balance, ArithmeticError> {
			Ok(balance / 100)
		}
	}

	pub struct IsLiquidToken;
	impl Contains<CurrencyId> for IsLiquidToken {
		fn contains(currency_id: &CurrencyId) -> bool {
			matches!(currency_id, &DOT)
		}
	}

	pub struct GetCurrencyId;
	impl Get<CurrencyId> for GetCurrencyId {
		fn get() -> CurrencyId {
			DOT
		}
	}

	type RebaseTokens = Combiner<
		AccountId,
		IsLiquidToken,
		Mapper<AccountId, Tokens, ConvertBalanceTest, Balance, GetCurrencyId>,
		Tokens,
	>;

	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100), (BOB, BTC, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(
				<RebaseTokens as fungibles::Inspect<AccountId>>::balance(DOT, &ALICE),
				10000
			);
			assert_eq!(
				<RebaseTokens as fungibles::Inspect<AccountId>>::total_issuance(DOT),
				20000
			);

			assert!(<Tokens as fungibles::Inspect<_>>::asset_exists(DOT));
			assert!(<Tokens as fungibles::Inspect<_>>::asset_exists(BTC));
			assert!(!<Tokens as fungibles::Inspect<_>>::asset_exists(ETH));
		});
}

#[test]
fn fungibles_transfers_convert_should_work() {
	pub struct ConvertBalanceTest;
	impl ConvertBalance<Balance, Balance> for ConvertBalanceTest {
		type AssetId = CurrencyId;
		fn convert_balance(balance: Balance, _asset_id: CurrencyId) -> Result<Balance, ArithmeticError> {
			Ok(balance * 100)
		}

		fn convert_balance_back(balance: Balance, _asset_id: CurrencyId) -> Result<Balance, ArithmeticError> {
			Ok(balance / 100)
		}
	}

	pub struct IsLiquidToken;
	impl Contains<CurrencyId> for IsLiquidToken {
		fn contains(currency_id: &CurrencyId) -> bool {
			matches!(currency_id, &DOT)
		}
	}

	pub struct GetCurrencyId;
	impl Get<CurrencyId> for GetCurrencyId {
		fn get() -> CurrencyId {
			DOT
		}
	}

	type RebaseTokens = Combiner<
		AccountId,
		IsLiquidToken,
		Mapper<AccountId, Tokens, ConvertBalanceTest, Balance, GetCurrencyId>,
		Tokens,
	>;

	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 300), (BOB, DOT, 200)])
		.build()
		.execute_with(|| {
			assert_ok!(<RebaseTokens as fungibles::Mutate<AccountId>>::transfer(
				DOT,
				&ALICE,
				&BOB,
				10000,
				Preservation::Protect
			));
			assert_eq!(
				<RebaseTokens as fungibles::Inspect<AccountId>>::balance(DOT, &ALICE),
				20000
			);
			assert_eq!(
				<RebaseTokens as fungibles::Inspect<AccountId>>::balance(DOT, &BOB),
				30000
			);
		});
}

#[test]
fn fungibles_mutate_convert_should_work() {
	pub struct ConvertBalanceTest;
	impl ConvertBalance<Balance, Balance> for ConvertBalanceTest {
		type AssetId = CurrencyId;
		fn convert_balance(balance: Balance, _asset_id: CurrencyId) -> Result<Balance, ArithmeticError> {
			Ok(balance * 100)
		}

		fn convert_balance_back(balance: Balance, _asset_id: CurrencyId) -> Result<Balance, ArithmeticError> {
			Ok(balance / 100)
		}
	}

	pub struct IsLiquidToken;
	impl Contains<CurrencyId> for IsLiquidToken {
		fn contains(currency_id: &CurrencyId) -> bool {
			matches!(currency_id, &DOT)
		}
	}

	pub struct GetCurrencyId;
	impl Get<CurrencyId> for GetCurrencyId {
		fn get() -> CurrencyId {
			DOT
		}
	}

	type RebaseTokens = Combiner<
		AccountId,
		IsLiquidToken,
		Mapper<AccountId, Tokens, ConvertBalanceTest, Balance, GetCurrencyId>,
		Tokens,
	>;

	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 300), (BOB, DOT, 200)])
		.build()
		.execute_with(|| {
			assert_ok!(<RebaseTokens as fungibles::Mutate<AccountId>>::mint_into(
				DOT, &ALICE, 10000
			));
			assert_ok!(<RebaseTokens as fungibles::Mutate<AccountId>>::burn_from(
				DOT,
				&BOB,
				10000,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Polite
			));
			assert_eq!(
				<RebaseTokens as fungibles::Inspect<AccountId>>::balance(DOT, &ALICE),
				40000
			);
			assert_eq!(
				<RebaseTokens as fungibles::Inspect<AccountId>>::balance(DOT, &BOB),
				10000
			);
		});
}
