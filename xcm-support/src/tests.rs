//! Unit tests for xcm-support implementations.

#![cfg(test)]

use super::*;

use orml_traits::{location::AbsoluteReserveProvider, location::RelativeLocations, ConcreteFungibleAsset};

#[derive(Debug, PartialEq, Eq)]
pub enum TestCurrencyId {
	TokenA,
	TokenB,
	RelayChainToken,
}

pub struct CurrencyIdConvert;
impl Convert<Location, Option<TestCurrencyId>> for CurrencyIdConvert {
	fn convert(l: Location) -> Option<TestCurrencyId> {
		use TestCurrencyId::*;

		if l == Location::parent() {
			return Some(RelayChainToken);
		}
		if l == Location::sibling_parachain_general_key(1, b"TokenA".to_vec().try_into().unwrap()) {
			return Some(TokenA);
		}
		if l == Location::sibling_parachain_general_key(2, b"TokenB".to_vec().try_into().unwrap()) {
			return Some(TokenB);
		}
		None
	}
}

type MatchesCurrencyId = IsNativeConcrete<TestCurrencyId, CurrencyIdConvert>;

#[test]
fn is_native_concrete_matches_native_currencies() {
	assert_eq!(
		MatchesCurrencyId::matches_fungible(&Asset::parent_asset(100)),
		Some(100),
	);

	assert_eq!(
		MatchesCurrencyId::matches_fungible(&Asset::sibling_parachain_asset(
			1,
			b"TokenA".to_vec().try_into().unwrap(),
			100
		)),
		Some(100),
	);

	assert_eq!(
		MatchesCurrencyId::matches_fungible(&Asset::sibling_parachain_asset(
			2,
			b"TokenB".to_vec().try_into().unwrap(),
			100
		)),
		Some(100),
	);
}

#[test]
fn is_native_concrete_does_not_matches_non_native_currencies() {
	assert!(
		<MatchesCurrencyId as MatchesFungible<u128>>::matches_fungible(&Asset::sibling_parachain_asset(
			2,
			b"TokenC".to_vec().try_into().unwrap(),
			100
		))
		.is_none()
	);
	assert!(
		<MatchesCurrencyId as MatchesFungible<u128>>::matches_fungible(&Asset::sibling_parachain_asset(
			1,
			b"TokenB".to_vec().try_into().unwrap(),
			100
		))
		.is_none()
	);
	assert!(<MatchesCurrencyId as MatchesFungible<u128>>::matches_fungible(&Asset {
		fun: Fungible(100),
		id: AssetId(Location::new(
			1,
			[Junction::from(
				sp_runtime::BoundedVec::try_from(b"TokenB".to_vec()).unwrap()
			)]
		)),
	})
	.is_none());
}

#[test]
fn multi_native_asset() {
	assert!(MultiNativeAsset::<AbsoluteReserveProvider>::contains(
		&Asset {
			fun: Fungible(10),
			id: AssetId(Location::parent())
		},
		&Parent.into()
	));
	assert!(MultiNativeAsset::<AbsoluteReserveProvider>::contains(
		&Asset::sibling_parachain_asset(1, b"TokenA".to_vec().try_into().unwrap(), 100),
		&Location::new(1, [Parachain(1)]),
	));
	assert!(!MultiNativeAsset::<AbsoluteReserveProvider>::contains(
		&Asset::sibling_parachain_asset(1, b"TokenA".to_vec().try_into().unwrap(), 100),
		&Location::parent(),
	));
}
