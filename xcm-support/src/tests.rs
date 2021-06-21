//! Unit tests for xcm-support implementations.

#![cfg(test)]

use super::*;

use xcm::v0::{Junction::*, MultiAsset::ConcreteFungible, MultiLocation::*};

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
		match l {
			X1(Parent) => Some(RelayChainToken),
			X3(Parent, Parachain(1), GeneralKey(k)) if k == token_a => Some(TokenA),
			X3(Parent, Parachain(2), GeneralKey(k)) if k == token_b => Some(TokenB),
			_ => None,
		}
	}
}

type MatchesCurrencyId = IsNativeConcrete<TestCurrencyId, CurrencyIdConvert>;

#[test]
fn is_native_concrete_matches_native_currencies() {
	assert_eq!(
		MatchesCurrencyId::matches_fungible(&ConcreteFungible {
			id: X1(Parent),
			amount: 100
		}),
		Some(100),
	);
	assert_eq!(
		MatchesCurrencyId::matches_fungible(&ConcreteFungible {
			id: X3(Parent, Parachain(1), GeneralKey("TokenA".into())),
			amount: 100
		}),
		Some(100),
	);
	assert_eq!(
		MatchesCurrencyId::matches_fungible(&ConcreteFungible {
			id: X3(Parent, Parachain(2), GeneralKey("TokenB".into())),
			amount: 100
		}),
		Some(100),
	);
}

#[test]
fn is_native_concrete_does_not_matches_non_native_currencies() {
	assert!(
		<MatchesCurrencyId as MatchesFungible<u128>>::matches_fungible(&ConcreteFungible {
			id: X3(Parent, Parachain(2), GeneralKey("TokenC".into())),
			amount: 100
		})
		.is_none()
	);
	assert!(
		<MatchesCurrencyId as MatchesFungible<u128>>::matches_fungible(&ConcreteFungible {
			id: X3(Parent, Parachain(1), GeneralKey("TokenB".into())),
			amount: 100
		})
		.is_none()
	);
	assert!(
		<MatchesCurrencyId as MatchesFungible<u128>>::matches_fungible(&ConcreteFungible {
			id: X1(GeneralKey("TokenB".into())),
			amount: 100
		})
		.is_none()
	);
}

#[test]
fn multi_native_asset() {
	assert!(MultiNativeAsset::filter_asset_location(
		&ConcreteFungible {
			id: Parent.into(),
			amount: 10,
		},
		&Parent.into()
	));
	assert!(MultiNativeAsset::filter_asset_location(
		&ConcreteFungible {
			id: X3(Parent, Parachain(1), GeneralKey("TokenA".into())),
			amount: 10,
		},
		&X2(Parent, Parachain(1)),
	));
	assert_eq!(
		MultiNativeAsset::filter_asset_location(
			&ConcreteFungible {
				id: X3(Parent, Parachain(1), GeneralKey("TokenA".into())),
				amount: 10,
			},
			&X1(Parent),
		),
		false
	);
}
