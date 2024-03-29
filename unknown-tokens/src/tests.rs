//! Unit tests for unknown tokens pallet.

#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_err, assert_ok};

const MOCK_RECIPIENT: Location = Location::parent();
const MOCK_CONCRETE_FUNGIBLE_ID: Location = Location::parent();

fn concrete_fungible(amount: u128) -> Asset {
	(MOCK_CONCRETE_FUNGIBLE_ID, amount).into()
}

#[test]
fn deposit_concrete_fungible_asset_works() {
	ExtBuilder.build().execute_with(|| {
		let asset = concrete_fungible(3);
		assert_ok!(UnknownTokens::deposit(&asset, &MOCK_RECIPIENT));
		assert_eq!(
			UnknownTokens::concrete_fungible_balances(&MOCK_RECIPIENT, &MOCK_CONCRETE_FUNGIBLE_ID),
			3
		);
		System::assert_last_event(RuntimeEvent::UnknownTokens(crate::Event::Deposited {
			asset,
			who: MOCK_RECIPIENT,
		}));

		// overflow case
		let max_asset = concrete_fungible(u128::max_value());
		assert_err!(
			UnknownTokens::deposit(&max_asset, &MOCK_RECIPIENT),
			Error::<Runtime>::BalanceOverflow
		);
	});
}

#[test]
fn deposit_unhandled_asset_should_fail() {
	ExtBuilder.build().execute_with(|| {
		assert_err!(
			UnknownTokens::deposit(
				&Asset {
					fun: NonFungible(Undefined),
					id: AssetId(Location::parent())
				},
				&MOCK_RECIPIENT
			),
			Error::<Runtime>::UnhandledAsset
		);
	});
}

#[test]
fn withdraw_concrete_fungible_asset_works() {
	ExtBuilder.build().execute_with(|| {
		ConcreteFungibleBalances::<Runtime>::insert(&MOCK_RECIPIENT, &MOCK_CONCRETE_FUNGIBLE_ID, 3);

		let asset = concrete_fungible(3);
		assert_ok!(UnknownTokens::withdraw(&asset, &MOCK_RECIPIENT));
		assert_eq!(
			UnknownTokens::concrete_fungible_balances(&MOCK_RECIPIENT, &MOCK_CONCRETE_FUNGIBLE_ID),
			0
		);
		System::assert_last_event(RuntimeEvent::UnknownTokens(crate::Event::Withdrawn {
			asset: asset.clone(),
			who: MOCK_RECIPIENT,
		}));

		// balance too low case
		assert_err!(
			UnknownTokens::withdraw(&asset, &MOCK_RECIPIENT),
			Error::<Runtime>::BalanceTooLow
		);
	});
}

#[test]
fn withdraw_unhandled_asset_should_fail() {
	ExtBuilder.build().execute_with(|| {
		assert_err!(
			UnknownTokens::withdraw(
				&Asset {
					fun: NonFungible(Undefined),
					id: AssetId(Location::parent())
				},
				&MOCK_RECIPIENT
			),
			Error::<Runtime>::UnhandledAsset
		);
	});
}
