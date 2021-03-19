//! Unit tests for xcm-support implementations.

#![cfg(test)]

use super::*;

use frame_support::parameter_types;
use sp_runtime::traits::{Convert, Identity};

#[derive(Debug, PartialEq, Eq)]
pub enum TestCurrencyId {
	TokenA,
	TokenB,
	RelayChainToken,
}
impl TryFrom<Vec<u8>> for TestCurrencyId {
	type Error = ();
	fn try_from(v: Vec<u8>) -> Result<TestCurrencyId, ()> {
		match v.as_slice() {
			[1] => Ok(TestCurrencyId::TokenA),
			[2] => Ok(TestCurrencyId::TokenB),
			[3] => Ok(TestCurrencyId::RelayChainToken),
			_ => Err(()),
		}
	}
}

type IdentityMatch = IsConcreteWithGeneralKey<TestCurrencyId, Identity>;

pub struct NativeToRelay;
impl Convert<u128, u128> for NativeToRelay {
	fn convert(val: u128) -> u128 {
		// native is 13
		// relay is 12
		val / 10
	}
}

type TenToOneMatch = IsConcreteWithGeneralKey<TestCurrencyId, NativeToRelay>;

parameter_types! {
	pub NativeOrmlTokens: BTreeSet<(Vec<u8>, MultiLocation)> = {
		let mut t = BTreeSet::new();
		t.insert((vec![1], (Junction::Parent, Junction::Parachain { id: 1 }).into()));
		t
	};

	pub const RelayChainCurrencyId: TestCurrencyId = TestCurrencyId::RelayChainToken;
}

type AssetFilter = NativePalletAssetOr<NativeOrmlTokens>;

type TestCurrencyIdConverter = CurrencyIdConverter<TestCurrencyId, RelayChainCurrencyId>;

#[test]
fn is_concrete_with_general_key_matches_relay_chain_token() {
	let relay_chain_asset = MultiAsset::ConcreteFungible {
		id: MultiLocation::X1(Junction::Parent),
		amount: 10,
	};
	assert_eq!(IdentityMatch::matches_fungible(&relay_chain_asset), Some(10));
	assert_eq!(TenToOneMatch::matches_fungible(&relay_chain_asset), Some(1));
}

#[test]
fn is_concrete_with_general_key_matches_parachain_token_with_general_key() {
	let token_a = MultiAsset::ConcreteFungible {
		id: MultiLocation::X3(
			Junction::Parent,
			Junction::Parachain { id: 1 },
			Junction::GeneralKey(vec![1]),
		),
		amount: 10,
	};
	let unknown_token = MultiAsset::ConcreteFungible {
		id: MultiLocation::X3(
			Junction::Parent,
			Junction::Parachain { id: 1 },
			Junction::GeneralKey(vec![100]),
		),
		amount: 10,
	};
	assert_eq!(IdentityMatch::matches_fungible(&token_a), Some(10));
	assert_eq!(
		<IdentityMatch as MatchesFungible<u128>>::matches_fungible(&unknown_token),
		None,
	);
}

#[test]
fn native_pallet_asset_or_can_filter_native_asset() {
	let token_a = MultiAsset::ConcreteFungible {
		id: MultiLocation::X2(Junction::Parent, Junction::Parachain { id: 1 }),
		amount: 10,
	};
	assert!(AssetFilter::filter_asset_location(
		&token_a,
		&MultiLocation::X2(Junction::Parent, Junction::Parachain { id: 1 }),
	));
}

#[test]
fn native_pallet_asset_or_can_filter_orml_tokens() {
	let token_a = MultiAsset::ConcreteFungible {
		id: MultiLocation::X3(
			Junction::Parent,
			Junction::Parachain { id: 1 },
			Junction::GeneralKey(vec![1]),
		),
		amount: 10,
	};
	// origin is different from concrete fungible id, thus it's not native.
	assert!(AssetFilter::filter_asset_location(
		&token_a,
		&MultiLocation::X2(Junction::Parent, Junction::Parachain { id: 1 }),
	));
}

#[test]
fn currency_id_converts_relay_chain_token() {
	let relay_chain_asset = MultiAsset::ConcreteFungible {
		id: MultiLocation::X1(Junction::Parent),
		amount: 10,
	};

	assert_eq!(
		TestCurrencyIdConverter::from_asset(&relay_chain_asset),
		Some(TestCurrencyId::RelayChainToken),
	);
}

#[test]
fn currency_id_converts_parachain_token() {
	let token_a = MultiAsset::ConcreteFungible {
		id: MultiLocation::X3(
			Junction::Parent,
			Junction::Parachain { id: 1 },
			Junction::GeneralKey(vec![1]),
		),
		amount: 10,
	};

	assert_eq!(
		TestCurrencyIdConverter::from_asset(&token_a),
		Some(TestCurrencyId::TokenA),
	);
}
