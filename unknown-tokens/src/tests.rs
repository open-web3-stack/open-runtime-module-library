//! Unit tests for unknown tokens pallet.

#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{assert_err, assert_ok};

const MOCK_RECIPIENT: MultiLocation = MultiLocation::parent();
const MOCK_CONCRETE_FUNGIBLE_ID: MultiLocation = MultiLocation::parent();

fn mock_abstract_fungible_id() -> [u8; 32] {
	[1; 32]
}

fn concrete_fungible(amount: u128) -> MultiAsset {
	(MOCK_CONCRETE_FUNGIBLE_ID, amount).into()
}

fn abstract_fungible(amount: u128) -> MultiAsset {
	(mock_abstract_fungible_id(), amount).into()
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
fn deposit_abstract_fungible_asset() {
	ExtBuilder.build().execute_with(|| {
		let asset = abstract_fungible(3);
		assert_ok!(UnknownTokens::deposit(&asset, &MOCK_RECIPIENT));
		assert_eq!(
			UnknownTokens::abstract_fungible_balances(&MOCK_RECIPIENT, &mock_abstract_fungible_id().to_vec()),
			3
		);
		System::assert_last_event(RuntimeEvent::UnknownTokens(crate::Event::Deposited {
			asset,
			who: MOCK_RECIPIENT,
		}));

		// overflow case
		let max_asset = abstract_fungible(u128::max_value());
		assert_err!(
			UnknownTokens::deposit(&max_asset, &MOCK_RECIPIENT),
			Error::<Runtime>::BalanceOverflow
		);
		assert_eq!(
			UnknownTokens::abstract_fungible_balances(&MOCK_RECIPIENT, &mock_abstract_fungible_id().to_vec()),
			3
		);
	});
}

#[test]
fn deposit_unhandled_asset_should_fail() {
	ExtBuilder.build().execute_with(|| {
		assert_err!(
			UnknownTokens::deposit(
				&MultiAsset {
					fun: NonFungible(Undefined),
					id: Concrete(MultiLocation::parent())
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
fn withdraw_abstract_fungible_asset_works() {
	ExtBuilder.build().execute_with(|| {
		AbstractFungibleBalances::<Runtime>::insert(&MOCK_RECIPIENT, &mock_abstract_fungible_id().to_vec(), 3);

		let asset = abstract_fungible(3);
		assert_ok!(UnknownTokens::withdraw(&asset, &MOCK_RECIPIENT));
		assert_eq!(
			UnknownTokens::abstract_fungible_balances(&MOCK_RECIPIENT, &mock_abstract_fungible_id().to_vec()),
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
				&MultiAsset {
					fun: NonFungible(Undefined),
					id: Concrete(MultiLocation::parent())
				},
				&MOCK_RECIPIENT
			),
			Error::<Runtime>::UnhandledAsset
		);
	});
}
