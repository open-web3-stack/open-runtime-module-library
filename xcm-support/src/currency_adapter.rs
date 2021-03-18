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
	/// Match fungible failed.
	MatchFungibleFailed,
	/// `MultiLocation` to `AccountId` Conversion failed.
	AccountIdConversionFailed,
	/// `CurrencyId` conversion failed.
	CurrencyIdConversionFailed,
}

impl From<Error> for XcmError {
	fn from(e: Error) -> Self {
		match e {
			Error::MatchFungibleFailed => XcmError::FailedToTransactAsset("MatchFungibleFailed"),
			Error::AccountIdConversionFailed => XcmError::FailedToTransactAsset("AccountIdConversionFailed"),
			Error::CurrencyIdConversionFailed => XcmError::FailedToTransactAsset("CurrencyIdConversionFailed"),
		}
	}
}

/// The `TransactAsset` implementation, to handle deposit/withdraw for XCM.
///
/// The implementation will try deposit or withdraw on unknown asset first, so
/// that detailed error info of known asset failure could be returned, if any.
/// Thus known asset deposit/withdraw failures imply unknown asset failures as
/// well.
pub struct MultiCurrencyAdapter<
	MultiCurrency,
	Matcher,
	AccountIdConverter,
	AccountId,
	CurrencyIdConverter,
	CurrencyId,
	UnknownAsset,
>(
	PhantomData<(
		MultiCurrency,
		Matcher,
		AccountIdConverter,
		AccountId,
		CurrencyIdConverter,
		CurrencyId,
		UnknownAsset,
	)>,
);

impl<
		MultiCurrency: orml_traits::MultiCurrency<AccountId, CurrencyId = CurrencyId>,
		Matcher: MatchesFungible<MultiCurrency::Balance>,
		AccountIdConverter: LocationConversion<AccountId>,
		AccountId: sp_std::fmt::Debug,
		CurrencyIdConverter: CurrencyIdConversion<CurrencyId>,
		CurrencyId: FullCodec + Eq + PartialEq + Copy + MaybeSerializeDeserialize + Debug,
		UnknownAsset: UnknownAssetT,
	> TransactAsset
	for MultiCurrencyAdapter<
		MultiCurrency,
		Matcher,
		AccountIdConverter,
		AccountId,
		CurrencyIdConverter,
		CurrencyId,
		UnknownAsset,
	>
{
	fn deposit_asset(asset: &MultiAsset, location: &MultiLocation) -> Result {
		UnknownAsset::deposit(asset, location).or_else(|_| {
			let who = AccountIdConverter::from_location(location)
				.ok_or_else(|| XcmError::from(Error::AccountIdConversionFailed))?;
			let currency_id = CurrencyIdConverter::from_asset(asset)
				.ok_or_else(|| XcmError::from(Error::CurrencyIdConversionFailed))?;
			let amount: MultiCurrency::Balance = Matcher::matches_fungible(&asset)
				.ok_or_else(|| XcmError::from(Error::MatchFungibleFailed))?
				.saturated_into();
			MultiCurrency::deposit(currency_id, &who, amount).map_err(|e| XcmError::FailedToTransactAsset(e.into()))
		})
	}

	fn withdraw_asset(asset: &MultiAsset, location: &MultiLocation) -> result::Result<MultiAsset, XcmError> {
		UnknownAsset::withdraw(asset, location).or_else(|_| {
			let who = AccountIdConverter::from_location(location)
				.ok_or_else(|| XcmError::from(Error::AccountIdConversionFailed))?;
			let currency_id = CurrencyIdConverter::from_asset(asset)
				.ok_or_else(|| XcmError::from(Error::CurrencyIdConversionFailed))?;
			let amount: MultiCurrency::Balance = Matcher::matches_fungible(&asset)
				.ok_or_else(|| XcmError::from(Error::MatchFungibleFailed))?
				.saturated_into();
			MultiCurrency::withdraw(currency_id, &who, amount).map_err(|e| XcmError::FailedToTransactAsset(e.into()))
		})?;

		Ok(asset.clone())
	}
}
