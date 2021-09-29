//! Unit tests for xcm-support implementations.

#![cfg(test)]

use super::*;

use orml_traits::{location::RelativeLocations, ConcreteFungibleAsset};

#[derive(Debug, PartialEq, Eq)]
pub enum TestCurrencyId {
	TokenA,
	TokenB,
	RelayChainToken,
}

pub struct CurrencyIdConvert;
impl Convert<MultiLocation, Option<TestCurrencyId>> for CurrencyIdConvert {
	fn convert(l: MultiLocation) -> Option<TestCurrencyId> {
		use TestCurrencyId::*;
		let token_a: Vec<u8> = "TokenA".into();
		let token_b: Vec<u8> = "TokenB".into();

		if l == MultiLocation::parent() {
			return Some(RelayChainToken);
		}
		if l == MultiLocation::sibling_parachain_general_key(1, token_a) {
			return Some(TokenA);
		}
		if l == MultiLocation::sibling_parachain_general_key(2, token_b) {
			return Some(TokenB);
		}
		None
	}
}

type MatchesCurrencyId = IsNativeConcrete<TestCurrencyId, CurrencyIdConvert>;

#[test]
fn is_native_concrete_matches_native_currencies() {
	assert_eq!(
		MatchesCurrencyId::matches_fungible(&MultiAsset::parent_asset(100)),
		Some(100),
	);

	assert_eq!(
		MatchesCurrencyId::matches_fungible(&MultiAsset::sibling_parachain_asset(1, "TokenA".into(), 100)),
		Some(100),
	);

	assert_eq!(
		MatchesCurrencyId::matches_fungible(&MultiAsset::sibling_parachain_asset(2, "TokenB".into(), 100)),
		Some(100),
	);
}

#[test]
fn is_native_concrete_does_not_matches_non_native_currencies() {
	assert!(
		<MatchesCurrencyId as MatchesFungible<u128>>::matches_fungible(&MultiAsset::sibling_parachain_asset(
			2,
			"TokenC".into(),
			100
		))
		.is_none()
	);
	assert!(
		<MatchesCurrencyId as MatchesFungible<u128>>::matches_fungible(&MultiAsset::sibling_parachain_asset(
			1,
			"TokenB".into(),
			100
		))
		.is_none()
	);
	assert!(
		<MatchesCurrencyId as MatchesFungible<u128>>::matches_fungible(&MultiAsset {
			fun: Fungible(100),
			id: Concrete(MultiLocation::new(1, X1(GeneralKey("TokenB".into())))),
		})
		.is_none()
	);
}

#[test]
fn multi_native_asset() {
	assert!(MultiNativeAsset::filter_asset_location(
		&MultiAsset {
			fun: Fungible(10),
			id: Concrete(MultiLocation::parent())
		},
		&Parent.into()
	));
	assert!(MultiNativeAsset::filter_asset_location(
		&MultiAsset::sibling_parachain_asset(1, "TokenA".into(), 100),
		&MultiLocation::new(1, X1(Parachain(1))),
	));
	assert!(!MultiNativeAsset::filter_asset_location(
		&MultiAsset::sibling_parachain_asset(1, "TokenA".into(), 100),
		&MultiLocation::parent(),
	));
}
