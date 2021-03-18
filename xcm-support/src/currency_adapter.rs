use codec::FullCodec;
use sp_runtime::traits::{MaybeSerializeDeserialize, SaturatedConversion};
use sp_std::{
	cmp::{Eq, PartialEq},
	fmt::Debug,
	marker::PhantomData,
	prelude::*,
	result,
};

use xcm::v0::{Error as XcmError, MultiAsset, MultiLocation, Result};
use xcm_executor::traits::{LocationConversion, MatchesFungible, TransactAsset};

use crate::{CurrencyIdConversion, UnknownAsset as UnknownAssetT};

/// Asset transaction errors.
enum Error {
	/// Failed to match fungible.
	FailedToMatchFungible,
	/// `MultiLocation` to `AccountId` Conversion failed.
	AccountIdConversionFailed,
	/// `CurrencyId` conversion failed.
	CurrencyIdConversionFailed,
}

impl From<Error> for XcmError {
	fn from(e: Error) -> Self {
		match e {
			Error::FailedToMatchFungible => XcmError::FailedToTransactAsset("FailedToMatchFungible"),
			Error::AccountIdConversionFailed => XcmError::FailedToTransactAsset("AccountIdConversionFailed"),
			Error::CurrencyIdConversionFailed => XcmError::FailedToTransactAsset("CurrencyIdConversionFailed"),
		}
	}
}

/// The `TransactAsset` implementation, to handle `MultiAsset` deposit/withdraw.
///
/// If the asset is known, deposit/withdraw will be handled by `MultiCurrency`,
/// else by `UnknownAsset` if unknown.
pub struct MultiCurrencyAdapter<
	MultiCurrency,
	UnknownAsset,
	Matcher,
	AccountIdConverter,
	AccountId,
	CurrencyIdConverter,
	CurrencyId,
>(
	PhantomData<(
		MultiCurrency,
		UnknownAsset,
		Matcher,
		AccountIdConverter,
		AccountId,
		CurrencyIdConverter,
		CurrencyId,
	)>,
);

impl<
		MultiCurrency: orml_traits::MultiCurrency<AccountId, CurrencyId = CurrencyId>,
		UnknownAsset: UnknownAssetT,
		Matcher: MatchesFungible<MultiCurrency::Balance>,
		AccountIdConverter: LocationConversion<AccountId>,
		AccountId: sp_std::fmt::Debug,
		CurrencyIdConverter: CurrencyIdConversion<CurrencyId>,
		CurrencyId: FullCodec + Eq + PartialEq + Copy + MaybeSerializeDeserialize + Debug,
	> TransactAsset
	for MultiCurrencyAdapter<
		MultiCurrency,
		UnknownAsset,
		Matcher,
		AccountIdConverter,
		AccountId,
		CurrencyIdConverter,
		CurrencyId,
	>
{
	fn deposit_asset(asset: &MultiAsset, location: &MultiLocation) -> Result {
		match (
			AccountIdConverter::from_location(location),
			CurrencyIdConverter::from_asset(asset),
			Matcher::matches_fungible(&asset),
		) {
			// known asset
			(Some(who), Some(currency_id), Some(amount)) => {
				MultiCurrency::deposit(currency_id, &who, amount).map_err(|e| XcmError::FailedToTransactAsset(e.into()))
			}
			// unknown asset
			_ => UnknownAsset::deposit(asset, location).map_err(|e| XcmError::FailedToTransactAsset(e.into())),
		}
	}

	fn withdraw_asset(asset: &MultiAsset, location: &MultiLocation) -> result::Result<MultiAsset, XcmError> {
		UnknownAsset::withdraw(asset, location).or_else(|_| {
			let who = AccountIdConverter::from_location(location)
				.ok_or_else(|| XcmError::from(Error::AccountIdConversionFailed))?;
			let currency_id = CurrencyIdConverter::from_asset(asset)
				.ok_or_else(|| XcmError::from(Error::CurrencyIdConversionFailed))?;
			let amount: MultiCurrency::Balance = Matcher::matches_fungible(&asset)
				.ok_or_else(|| XcmError::from(Error::FailedToMatchFungible))?
				.saturated_into();
			MultiCurrency::withdraw(currency_id, &who, amount).map_err(|e| XcmError::FailedToTransactAsset(e.into()))
		})?;

		Ok(asset.clone())
	}
}
