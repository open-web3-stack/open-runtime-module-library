//! # XCM Support Module.
//!
//! ## Overview
//!
//! The XCM support module provides supporting traits, types and
//! implementations, to support cross-chain message(XCM) integration with ORML
//! modules.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::{dispatch::DispatchResult, traits::ContainsPair};
use sp_runtime::{
	traits::{CheckedConversion, Convert},
	DispatchError,
};
use sp_std::marker::PhantomData;

use xcm::v4::prelude::*;
use xcm_executor::traits::MatchesFungible;

use orml_traits::{location::Reserve, GetByKey};

pub use currency_adapter::{DepositToAlternative, MultiCurrencyAdapter, OnDepositFail};

mod currency_adapter;

mod tests;

/// A `MatchesFungible` implementation. It matches concrete fungible assets
/// whose `id` could be converted into `CurrencyId`.
pub struct IsNativeConcrete<CurrencyId, CurrencyIdConvert>(PhantomData<(CurrencyId, CurrencyIdConvert)>);
impl<CurrencyId, CurrencyIdConvert, Amount> MatchesFungible<Amount> for IsNativeConcrete<CurrencyId, CurrencyIdConvert>
where
	CurrencyIdConvert: Convert<Location, Option<CurrencyId>>,
	Amount: TryFrom<u128>,
{
	fn matches_fungible(a: &Asset) -> Option<Amount> {
		if let (Fungible(ref amount), AssetId(location)) = (&a.fun, &a.id) {
			if CurrencyIdConvert::convert(location.clone()).is_some() {
				return CheckedConversion::checked_from(*amount);
			}
		}
		None
	}
}

/// A `ContainsPair` implementation. Filters multi native assets whose
/// reserve is same with `origin`.
pub struct MultiNativeAsset<ReserveProvider>(PhantomData<ReserveProvider>);
impl<ReserveProvider> ContainsPair<Asset, Location> for MultiNativeAsset<ReserveProvider>
where
	ReserveProvider: Reserve,
{
	fn contains(asset: &Asset, origin: &Location) -> bool {
		if let Some(ref reserve) = ReserveProvider::reserve(asset) {
			if reserve == origin {
				return true;
			}
		}
		false
	}
}

/// Handlers unknown asset deposit and withdraw.
pub trait UnknownAsset {
	/// Deposit unknown asset.
	fn deposit(asset: &Asset, to: &Location) -> DispatchResult;

	/// Withdraw unknown asset.
	fn withdraw(asset: &Asset, from: &Location) -> DispatchResult;
}

const NO_UNKNOWN_ASSET_IMPL: &str = "NoUnknownAssetImpl";

impl UnknownAsset for () {
	fn deposit(_asset: &Asset, _to: &Location) -> DispatchResult {
		Err(DispatchError::Other(NO_UNKNOWN_ASSET_IMPL))
	}
	fn withdraw(_asset: &Asset, _from: &Location) -> DispatchResult {
		Err(DispatchError::Other(NO_UNKNOWN_ASSET_IMPL))
	}
}

// Default implementation for xTokens::MinXcmFee
pub struct DisabledParachainFee;
impl GetByKey<Location, Option<u128>> for DisabledParachainFee {
	fn get(_key: &Location) -> Option<u128> {
		None
	}
}
