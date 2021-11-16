//! Unit tests for xcm-support implementations.

#![cfg(test)]

use super::*;

use frame_support::{pallet_prelude::Encode, parameter_types};
use orml_traits::{location::RelativeLocations, ConcreteFungibleAsset};
use sp_runtime::AccountId32;

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

#[test]
fn relay_account_convert() {
	use xcm_executor::traits::Convert;

	parameter_types! {
		const RelayNetwork: NetworkId = NetworkId::Any;
	}
	let destination: MultiLocation = (
		Parent,
		Junction::AccountId32 {
			network: NetworkId::Any,
			id: [0; 32],
		},
	)
		.into();
	let account: Result<AccountId32, MultiLocation> =
		RelayChainAccountId32Aliases::<RelayNetwork, AccountId32>::convert(destination);
	assert_eq!(account, Ok(AccountId32::new([0; 32])));
}

#[test]
fn allow_relayed_paid_execution_works() {
	parameter_types! {
		const RelayNetwork: NetworkId = NetworkId::Any;
	}
	let assets: MultiAsset = (Parent, 1000).into();
	let mut xcm = Xcm::<()>(vec![
		DescendOrigin(X1(Junction::AccountId32 {
			network: NetworkId::Any,
			id: [0; 32],
		})),
		WithdrawAsset(assets.clone().into()),
		BuyExecution {
			fees: assets,
			weight_limit: Limited(1000),
		},
		Transact {
			origin_type: OriginKind::SovereignAccount,
			require_weight_at_most: 1000 as u64,
			call: Encode::encode(&100).into(),
		},
	]);
	let r = AllowRelayedPaidExecutionFromParent::<RelayNetwork>::should_execute(&(Parent.into()), &mut xcm, 100, &mut 100);
	assert_eq!(r, Ok(()));
}
