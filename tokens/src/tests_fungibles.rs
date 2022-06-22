//! Unit tests for the tokens module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;

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
				<Tokens as fungibles::Inspect<_>>::reducible_balance(DOT, &ALICE, true),
				98
			);
			assert_ok!(<Tokens as fungibles::Inspect<_>>::can_deposit(DOT, &ALICE, 1, false).into_result());
			assert_ok!(<Tokens as fungibles::Inspect<_>>::can_withdraw(DOT, &ALICE, 1).into_result());
		});
}

#[test]
fn fungibles_mutate_trait_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_ok!(<Tokens as fungibles::Mutate<_>>::mint_into(DOT, &ALICE, 10));
		assert_eq!(<Tokens as fungibles::Mutate<_>>::burn_from(DOT, &ALICE, 8), Ok(8));
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
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 100);
			assert_ok!(<Tokens as fungibles::Unbalanced<_>>::set_balance(DOT, &ALICE, 10));
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &ALICE), 10);

			assert_eq!(<Tokens as fungibles::Inspect<_>>::total_issuance(DOT), 100);
			<Tokens as fungibles::Unbalanced<_>>::set_total_issuance(DOT, 10);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::total_issuance(DOT), 10);
		});
}

#[test]
fn fungibles_inspect_hold_trait_should_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_eq!(<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, &ALICE), 0);
			assert!(<Tokens as fungibles::InspectHold<_>>::can_hold(DOT, &ALICE, 50));
			assert!(!<Tokens as fungibles::InspectHold<_>>::can_hold(DOT, &ALICE, 100));
		});
}

#[test]
fn fungibles_mutate_hold_trait_should_work() {
	ExtBuilder::default()
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
		.build()
		.execute_with(|| {
			assert_noop!(
				<Tokens as fungibles::MutateHold<_>>::hold(DOT, &ALICE, 200),
				Error::<Runtime>::BalanceTooLow
			);
			assert_eq!(<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, &ALICE), 0);
			assert_ok!(<Tokens as fungibles::MutateHold<_>>::hold(DOT, &ALICE, 100));
			assert_eq!(<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, &ALICE), 100);
			assert_eq!(
				<Tokens as fungibles::MutateHold<_>>::release(DOT, &ALICE, 40, false),
				Ok(40)
			);
			assert_eq!(<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, &ALICE), 60);

			// exceed hold amount when not in best_effort
			assert_noop!(
				<Tokens as fungibles::MutateHold<_>>::release(DOT, &ALICE, 61, false),
				Error::<Runtime>::BalanceTooLow
			);

			// exceed hold amount when in best_effort
			assert_eq!(
				<Tokens as fungibles::MutateHold<_>>::release(DOT, &ALICE, 61, true),
				Ok(60)
			);
			assert_eq!(<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, &ALICE), 0);

			assert_ok!(<Tokens as fungibles::MutateHold<_>>::hold(DOT, &ALICE, 70));
			assert_eq!(<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, &ALICE), 70);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &BOB), 100);
			assert_eq!(<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, &BOB), 0);
			assert_eq!(
				<Tokens as fungibles::MutateHold<_>>::transfer_held(DOT, &ALICE, &BOB, 5, false, false),
				Ok(5)
			);
			assert_eq!(<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, &ALICE), 65);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &BOB), 105);
			assert_eq!(<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, &BOB), 0);
			assert_eq!(
				<Tokens as fungibles::MutateHold<_>>::transfer_held(DOT, &ALICE, &BOB, 5, false, true),
				Ok(5)
			);
			assert_eq!(<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, &ALICE), 60);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &BOB), 110);
			assert_eq!(<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, &BOB), 5);

			// exceed hold amount when not in best_effort
			assert_noop!(
				<Tokens as fungibles::MutateHold<_>>::transfer_held(DOT, &ALICE, &BOB, 61, false, true),
				Error::<Runtime>::BalanceTooLow
			);

			// exceed hold amount when in best_effort
			assert_eq!(
				<Tokens as fungibles::MutateHold<_>>::transfer_held(DOT, &ALICE, &BOB, 61, true, true),
				Ok(60)
			);
			assert_eq!(<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, &ALICE), 0);
			assert_eq!(<Tokens as fungibles::Inspect<_>>::balance(DOT, &BOB), 170);
			assert_eq!(<Tokens as fungibles::InspectHold<_>>::balance_on_hold(DOT, &BOB), 65);
		});
}

#[test]
fn fungibles_inspect_convert_should_work() {
	pub struct ConvertBalanceTest;
	impl ConvertBalance<Balance, Balance> for ConvertBalanceTest {
		type AssetId = CurrencyId;
		fn convert_balance(balance: Balance, _asset_id: CurrencyId) -> Balance {
			balance * 100
		}

		fn convert_balance_back(balance: Balance, _asset_id: CurrencyId) -> Balance {
			balance / 100
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
		.balances(vec![(ALICE, DOT, 100), (BOB, DOT, 100)])
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
		});
}

#[test]
fn fungibles_transfers_convert_should_work() {
	pub struct ConvertBalanceTest;
	impl ConvertBalance<Balance, Balance> for ConvertBalanceTest {
		type AssetId = CurrencyId;
		fn convert_balance(balance: Balance, _asset_id: CurrencyId) -> Balance {
			balance * 100
		}

		fn convert_balance_back(balance: Balance, _asset_id: CurrencyId) -> Balance {
			balance / 100
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
			assert_ok!(<RebaseTokens as fungibles::Transfer<AccountId>>::transfer(
				DOT, &ALICE, &BOB, 10000, true
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
		fn convert_balance(balance: Balance, _asset_id: CurrencyId) -> Balance {
			balance * 100
		}

		fn convert_balance_back(balance: Balance, _asset_id: CurrencyId) -> Balance {
			balance / 100
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
				DOT, &BOB, 10000
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
