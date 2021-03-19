//! # XCM Support Module.
//!
//! ## Overview
//!
//! The XCM support module provides supporting traits, types and
//! implementations, to support cross-chain message(XCM) integration with ORML
//! modules.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{
	dispatch::{DispatchError, DispatchResult},
	traits::Get,
};
use sp_runtime::traits::{CheckedConversion, Convert};
use sp_std::{
	collections::btree_set::BTreeSet,
	convert::{TryFrom, TryInto},
	marker::PhantomData,
	prelude::*,
};

use xcm::v0::{Junction, MultiAsset, MultiLocation, Xcm};
use xcm_executor::traits::{FilterAssetLocation, MatchesFungible, NativeAsset};

pub use currency_adapter::MultiCurrencyAdapter;

mod currency_adapter;

mod tests;

/// The XCM handler to execute XCM locally.
pub trait XcmHandler<AccountId> {
	fn execute_xcm(origin: AccountId, xcm: Xcm) -> DispatchResult;
}

/// Convert `MultiAsset` to `CurrencyId`.
pub trait CurrencyIdConversion<CurrencyId> {
	/// Get `CurrencyId` from `MultiAsset`. Returns `None` if conversion failed.
	fn from_asset(asset: &MultiAsset) -> Option<CurrencyId>;
}

/// A `MatchesFungible` implementation. It matches relay chain tokens or
/// parachain tokens that could be decoded from a general key.
pub struct IsConcreteWithGeneralKey<CurrencyId, FromRelayChainBalance>(
	PhantomData<(CurrencyId, FromRelayChainBalance)>,
);
impl<CurrencyId, B, FromRelayChainBalance> MatchesFungible<B>
	for IsConcreteWithGeneralKey<CurrencyId, FromRelayChainBalance>
where
	CurrencyId: TryFrom<Vec<u8>>,
	B: TryFrom<u128>,
	FromRelayChainBalance: Convert<u128, u128>,
{
	fn matches_fungible(a: &MultiAsset) -> Option<B> {
		if let MultiAsset::ConcreteFungible { id, amount } = a {
			if id == &MultiLocation::X1(Junction::Parent) {
				// Convert relay chain decimals to local chain
				let local_amount = FromRelayChainBalance::convert(*amount);
				return CheckedConversion::checked_from(local_amount);
			}
			if let Some(Junction::GeneralKey(key)) = id.last() {
				if TryInto::<CurrencyId>::try_into(key.clone()).is_ok() {
					return CheckedConversion::checked_from(*amount);
				}
			}
		}
		None
	}
}

/// A `FilterAssetLocation` implementation. Filters native assets and ORML
/// tokens via provided general key to `MultiLocation` pairs.
pub struct NativePalletAssetOr<Pairs>(PhantomData<Pairs>);
impl<Pairs: Get<BTreeSet<(Vec<u8>, MultiLocation)>>> FilterAssetLocation for NativePalletAssetOr<Pairs> {
	fn filter_asset_location(asset: &MultiAsset, origin: &MultiLocation) -> bool {
		if NativeAsset::filter_asset_location(asset, origin) {
			return true;
		}

		// native orml-tokens with a general key
		if let MultiAsset::ConcreteFungible { ref id, .. } = asset {
			if let Some(Junction::GeneralKey(key)) = id.last() {
				return Pairs::get().contains(&(key.clone(), origin.clone()));
			}
		}

		false
	}
}

/// `CurrencyIdConversion` implementation. Converts relay chain tokens, or
/// parachain tokens that could be decoded from a general key.
pub struct CurrencyIdConverter<CurrencyId, RelayChainCurrencyId>(
	PhantomData<CurrencyId>,
	PhantomData<RelayChainCurrencyId>,
);
impl<CurrencyId, RelayChainCurrencyId> CurrencyIdConversion<CurrencyId>
	for CurrencyIdConverter<CurrencyId, RelayChainCurrencyId>
where
	CurrencyId: TryFrom<Vec<u8>>,
	RelayChainCurrencyId: Get<CurrencyId>,
{
	fn from_asset(asset: &MultiAsset) -> Option<CurrencyId> {
		if let MultiAsset::ConcreteFungible { id: location, .. } = asset {
			if location == &MultiLocation::X1(Junction::Parent) {
				return Some(RelayChainCurrencyId::get());
			}
			if let Some(Junction::GeneralKey(key)) = location.last() {
				return CurrencyId::try_from(key.clone()).ok();
			}
		}
		None
	}
}

/// Handlers unknown asset deposit and withdraw.
pub trait UnknownAsset {
	/// Deposit unknown asset.
	fn deposit(asset: &MultiAsset, to: &MultiLocation) -> DispatchResult;

	/// Withdraw unknown asset.
	fn withdraw(asset: &MultiAsset, from: &MultiLocation) -> DispatchResult;
}

const NO_UNKNOWN_ASSET_IMPL: &str = "NoUnknownAssetImpl";

impl UnknownAsset for () {
	fn deposit(_asset: &MultiAsset, _to: &MultiLocation) -> DispatchResult {
		Err(DispatchError::Other(NO_UNKNOWN_ASSET_IMPL))
	}
	fn withdraw(_asset: &MultiAsset, _from: &MultiLocation) -> DispatchResult {
		Err(DispatchError::Other(NO_UNKNOWN_ASSET_IMPL))
	}
}
