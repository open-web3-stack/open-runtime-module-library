//! Unit tests for xcm-support implementations.

#![cfg(test)]

use super::*;

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
		if l == MultiLocation::new(1, X2(Parachain(1), GeneralKey(token_a))) {
			return Some(TokenA);
		}
		if l == MultiLocation::new(1, X2(Parachain(2), GeneralKey(token_b))) {
			return Some(TokenB);
		}
		None
	}
}

type MatchesCurrencyId = IsNativeConcrete<TestCurrencyId, CurrencyIdConvert>;

#[test]
fn is_native_concrete_matches_native_currencies() {
	assert_eq!(
		MatchesCurrencyId::matches_fungible(&MultiAsset {
			fun: Fungible(100),
			id: Concrete(MultiLocation::parent()),
		}),
		Some(100),
	);

	assert_eq!(
		MatchesCurrencyId::matches_fungible(&MultiAsset {
			fun: Fungible(100),
			id: Concrete(MultiLocation::new(1, X2(Parachain(1), GeneralKey("TokenA".into())))),
		}),
		Some(100),
	);
	assert_eq!(
		MatchesCurrencyId::matches_fungible(&MultiAsset {
			fun: Fungible(100),
			id: Concrete(MultiLocation::new(1, X2(Parachain(2), GeneralKey("TokenB".into())))),
		}),
		Some(100),
	);
}

#[test]
fn is_native_concrete_does_not_matches_non_native_currencies() {
	assert!(
		<MatchesCurrencyId as MatchesFungible<u128>>::matches_fungible(&MultiAsset {
			fun: Fungible(100),
			id: Concrete(MultiLocation::new(1, X2(Parachain(2), GeneralKey("TokenC".into())))),
		})
		.is_none()
	);
	assert!(
		<MatchesCurrencyId as MatchesFungible<u128>>::matches_fungible(&MultiAsset {
			fun: Fungible(100),
			id: Concrete(MultiLocation::new(1, X2(Parachain(1), GeneralKey("TokenB".into())))),
		})
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
		&MultiAsset {
			fun: Fungible(100),
			id: Concrete(MultiLocation::new(1, X2(Parachain(1), GeneralKey("TokenA".into())))),
		},
		&MultiLocation::new(1, X1(Parachain(1))),
	));
	assert!(!MultiNativeAsset::filter_asset_location(
		&MultiAsset {
			fun: Fungible(100),
			id: Concrete(MultiLocation::new(1, X2(Parachain(1), GeneralKey("TokenA".into())))),
		},
		&MultiLocation::parent(),
	));
}
