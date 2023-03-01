//! Unit tests for unknown tokens pallet.

#![cfg(test)]

use super::*;
use mock::*;

use frame_support::{
	assert_err, assert_ok,
	storage::migration::{get_storage_value, put_storage_value},
	traits::OnRuntimeUpgrade,
};

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

#[test]
fn from_unversioned_to_v2_storage() {
	ExtBuilder.build().execute_with(|| {
		fn blake2_128_concat(d: &[u8]) -> Vec<u8> {
			let mut v = sp_io::hashing::blake2_128(d).to_vec();
			v.extend_from_slice(d);
			v
		}

		// StorageVersion is 0 before migration
		assert_eq!(StorageVersion::get::<Pallet<Runtime>>(), 0);

		// V2 `ConcreteFungibleBalances` key
		let mut old_concrete_key = Vec::new();
		old_concrete_key.extend_from_slice(
			&xcm::v2::MultiLocation::new(
				0,
				xcm::v2::Junctions::X1(xcm::v2::Junction::GeneralKey(vec![0].try_into().unwrap())),
			)
			.using_encoded(blake2_128_concat),
		);
		old_concrete_key.extend_from_slice(&xcm::v2::MultiLocation::here().using_encoded(blake2_128_concat));

		let balance = 55u128;

		put_storage_value(
			b"UnknownTokens",
			b"ConcreteFungibleBalances",
			&old_concrete_key,
			balance,
		);

		// V2 `AbstractFungibleBalances` key
		let mut old_abstract_key = Vec::new();
		old_abstract_key.extend_from_slice(
			&xcm::v2::MultiLocation::new(
				0,
				xcm::v2::Junctions::X1(xcm::v2::Junction::GeneralKey(vec![0].try_into().unwrap())),
			)
			.using_encoded(blake2_128_concat),
		);
		old_abstract_key.extend_from_slice(&vec![1].using_encoded(blake2_128_concat));

		let balance = 77u128;

		put_storage_value(
			b"UnknownTokens",
			b"AbstractFungibleBalances",
			&old_abstract_key,
			balance,
		);

		// V3 storage keys
		let new_concrete_k1 = MultiLocation::new(0, X1(Junction::from(BoundedVec::try_from(vec![0]).unwrap())));
		let new_concrete_k2 = MultiLocation::here();
		let new_abstract_k1 = MultiLocation::new(0, X1(Junction::from(BoundedVec::try_from(vec![0]).unwrap())));
		let new_abstract_k2 = vec![1];

		// Assert new StorageKey still does not exist
		assert_eq!(
			UnknownTokens::concrete_fungible_balances(new_concrete_k1, new_concrete_k2),
			0
		);
		assert_eq!(
			UnknownTokens::abstract_fungible_balances(new_abstract_k1, new_abstract_k2.clone()),
			0
		);

		// Migrate
		crate::Migration::<Runtime>::on_runtime_upgrade();

		// StorageVersion is 2 after migration
		assert_eq!(StorageVersion::get::<Pallet<Runtime>>(), 2);

		// Assert the StorageKey exists and has been migrated to xcm::v3
		assert_eq!(
			UnknownTokens::concrete_fungible_balances(new_concrete_k1, new_concrete_k2),
			55
		);
		assert_eq!(
			UnknownTokens::abstract_fungible_balances(new_abstract_k1, new_abstract_k2),
			77
		);

		// Assert the old concrete key does not exist anymore
		assert!(get_storage_value::<u128>(b"UnknownTokens", b"ConcreteFungibleBalances", &old_concrete_key,).is_none());

		// Assert the old abstract key does not exist anymore
		assert!(get_storage_value::<u128>(b"UnknownTokens", b"AbstractFungibleBalances", &old_concrete_key,).is_none());

		// Assert further calls are no-op
		assert_eq!(crate::Migration::<Runtime>::on_runtime_upgrade(), Weight::zero());
	});
}
