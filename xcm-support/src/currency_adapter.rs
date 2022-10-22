use codec::FullCodec;
use frame_support::traits::Get;
use sp_runtime::{
	traits::{Convert, MaybeSerializeDeserialize, SaturatedConversion},
	DispatchError,
};

use sp_std::{
	cmp::{Eq, PartialEq},
	fmt::Debug,
	marker::PhantomData,
	prelude::*,
	result,
};

use xcm::latest::{Error as XcmError, MultiAsset, MultiLocation, Result};
use xcm_executor::{
	traits::{Convert as MoreConvert, MatchesFungible, TransactAsset},
	Assets,
};

use crate::UnknownAsset as UnknownAssetT;

/// Asset transaction errors.
enum Error {
	/// Failed to match fungible.
	FailedToMatchFungible,
	/// `MultiLocation` to `AccountId` Conversion failed.
	AccountIdConversionFailed,
	/// `CurrencyId` conversion failed.
	CurrencyIdConversionFailed,
	/// Failed to normalize to desired decimals
	AmountNormalizationFailed,
}

impl From<Error> for XcmError {
	fn from(e: Error) -> Self {
		match e {
			Error::FailedToMatchFungible => XcmError::FailedToTransactAsset("FailedToMatchFungible"),
			Error::AccountIdConversionFailed => XcmError::FailedToTransactAsset("AccountIdConversionFailed"),
			Error::CurrencyIdConversionFailed => XcmError::FailedToTransactAsset("CurrencyIdConversionFailed"),
			Error::AmountNormalizationFailed => XcmError::FailedToTransactAsset("AmountNormalizationFailed"),
		}
	}
}

/// Deposit errors handler for `TransactAsset` implementations. Default impl for
/// `()` returns an `XcmError::FailedToTransactAsset` error.
pub trait OnDepositFail<CurrencyId, AccountId, Balance> {
	/// Called on deposit errors with a specific `currency_id`.
	fn on_deposit_currency_fail(
		err: DispatchError,
		currency_id: CurrencyId,
		who: &AccountId,
		amount: Balance,
	) -> Result;

	/// Called on unknown asset deposit errors.
	fn on_deposit_unknown_asset_fail(err: DispatchError, _asset: &MultiAsset, _location: &MultiLocation) -> Result {
		Err(XcmError::FailedToTransactAsset(err.into()))
	}
}

impl<CurrencyId, AccountId, Balance> OnDepositFail<CurrencyId, AccountId, Balance> for () {
	fn on_deposit_currency_fail(
		err: DispatchError,
		_currency_id: CurrencyId,
		_who: &AccountId,
		_amount: Balance,
	) -> Result {
		Err(XcmError::FailedToTransactAsset(err.into()))
	}
}

/// `OnDepositFail` impl, will deposit known currencies to an alternative
/// account.
pub struct DepositToAlternative<Alternative, MultiCurrency, CurrencyId, AccountId, Balance>(
	PhantomData<(Alternative, MultiCurrency, CurrencyId, AccountId, Balance)>,
);
impl<
		Alternative: Get<AccountId>,
		MultiCurrency: orml_traits::MultiCurrency<AccountId, CurrencyId = CurrencyId, Balance = Balance>,
		AccountId: sp_std::fmt::Debug + Clone,
		CurrencyId: FullCodec + Eq + PartialEq + Copy + MaybeSerializeDeserialize + Debug,
		Balance,
	> OnDepositFail<CurrencyId, AccountId, Balance>
	for DepositToAlternative<Alternative, MultiCurrency, CurrencyId, AccountId, Balance>
{
	fn on_deposit_currency_fail(
		_err: DispatchError,
		currency_id: CurrencyId,
		_who: &AccountId,
		amount: Balance,
	) -> Result {
		MultiCurrency::deposit(currency_id, &Alternative::get(), amount)
			.map_err(|e| XcmError::FailedToTransactAsset(e.into()))
	}
}

/// do not consider normalization needed
pub struct EqualDecimalsNormalizer {}
impl<CurrencyId> orml_traits::GetByKey<CurrencyId, i8> for EqualDecimalsNormalizer {
	fn get(_: &CurrencyId) -> i8 {
		0
	}
}

/// The `TransactAsset` implementation, to handle `MultiAsset` deposit/withdraw.
/// Note that teleport related functions are unimplemented.
///
/// Methods of `DepositFailureHandler` would be called on multi-currency deposit
/// errors.
///
/// If the asset is known, deposit/withdraw will be handled by `MultiCurrency`,
/// else by `UnknownAsset` if unknown.
#[allow(clippy::type_complexity)]
pub struct MultiCurrencyAdapter<
	MultiCurrency,
	UnknownAsset,
	Match,
	AccountId,
	AccountIdConvert,
	CurrencyId,
	CurrencyIdConvert,
	DepositFailureHandler,
	Normalizer,
>(
	PhantomData<(
		MultiCurrency,
		UnknownAsset,
		Match,
		AccountId,
		AccountIdConvert,
		CurrencyId,
		CurrencyIdConvert,
		DepositFailureHandler,
		Normalizer,
	)>,
);

impl<
		MultiCurrency,
		UnknownAsset,
		Match,
		AccountId,
		AccountIdConvert,
		CurrencyId,
		CurrencyIdConvert,
		DepositFailureHandler,
		Normalizer,
	>
	MultiCurrencyAdapter<
		MultiCurrency,
		UnknownAsset,
		Match,
		AccountId,
		AccountIdConvert,
		CurrencyId,
		CurrencyIdConvert,
		DepositFailureHandler,
		Normalizer,
	> where
	MultiCurrency: orml_traits::MultiCurrency<AccountId, CurrencyId = CurrencyId>,
	CurrencyId: FullCodec + Eq + PartialEq + Copy + MaybeSerializeDeserialize + Debug,
	Normalizer: orml_traits::GetByKey<CurrencyId, i8>,
{
	fn normalize(
		currency_id: CurrencyId,
		amount: <MultiCurrency as orml_traits::MultiCurrency<AccountId>>::Balance,
	) -> sp_std::result::Result<<MultiCurrency as orml_traits::MultiCurrency<AccountId>>::Balance, XcmError> {
		let decimals = Normalizer::get(&currency_id);
		let amount = if decimals == 0 {
			amount
		} else if decimals > 0 {
			num_traits::pow::checked_pow(amount, decimals as usize)
				.ok_or_else(|| XcmError::from(Error::AmountNormalizationFailed))?
		} else {
			// may be risky to reduce to lost precision until proven it is safe
			Err(XcmError::from(Error::AmountNormalizationFailed))?
		};
		Ok(amount)
	}
}

impl<
		MultiCurrency: orml_traits::MultiCurrency<AccountId, CurrencyId = CurrencyId>,
		UnknownAsset: UnknownAssetT,
		Match: MatchesFungible<MultiCurrency::Balance>,
		AccountId: sp_std::fmt::Debug + Clone,
		AccountIdConvert: MoreConvert<MultiLocation, AccountId>,
		CurrencyId: FullCodec + Eq + PartialEq + Copy + MaybeSerializeDeserialize + Debug,
		CurrencyIdConvert: Convert<MultiAsset, Option<CurrencyId>>,
		DepositFailureHandler: OnDepositFail<CurrencyId, AccountId, MultiCurrency::Balance>,
		Normalizer: orml_traits::GetByKey<CurrencyId, i8>,
	> TransactAsset
	for MultiCurrencyAdapter<
		MultiCurrency,
		UnknownAsset,
		Match,
		AccountId,
		AccountIdConvert,
		CurrencyId,
		CurrencyIdConvert,
		DepositFailureHandler,
		Normalizer,
	>
{
	fn deposit_asset(asset: &MultiAsset, location: &MultiLocation) -> Result {
		match (
			AccountIdConvert::convert_ref(location),
			CurrencyIdConvert::convert(asset.clone()),
			Match::matches_fungible(asset),
		) {
			// known asset
			(Ok(who), Some(currency_id), Some(amount)) => {
				let amount = Self::normalize(currency_id, amount)?;
				MultiCurrency::deposit(currency_id, &who, amount)
					.or_else(|err| DepositFailureHandler::on_deposit_currency_fail(err, currency_id, &who, amount))
			}

			// unknown asset
			_ => UnknownAsset::deposit(asset, location)
				.or_else(|err| DepositFailureHandler::on_deposit_unknown_asset_fail(err, asset, location)),
		}
	}

	fn withdraw_asset(asset: &MultiAsset, location: &MultiLocation) -> result::Result<Assets, XcmError> {
		UnknownAsset::withdraw(asset, location).or_else(|_| {
			let who = AccountIdConvert::convert_ref(location)
				.map_err(|_| XcmError::from(Error::AccountIdConversionFailed))?;
			let currency_id = CurrencyIdConvert::convert(asset.clone())
				.ok_or_else(|| XcmError::from(Error::CurrencyIdConversionFailed))?;
			let amount: MultiCurrency::Balance = Match::matches_fungible(asset)
				.ok_or_else(|| XcmError::from(Error::FailedToMatchFungible))?
				.saturated_into();
			MultiCurrency::withdraw(currency_id, &who, amount).map_err(|e| XcmError::FailedToTransactAsset(e.into()))
		})?;

		Ok(asset.clone().into())
	}

	fn transfer_asset(
		asset: &MultiAsset,
		from: &MultiLocation,
		to: &MultiLocation,
	) -> result::Result<Assets, XcmError> {
		let from_account =
			AccountIdConvert::convert_ref(from).map_err(|_| XcmError::from(Error::AccountIdConversionFailed))?;
		let to_account =
			AccountIdConvert::convert_ref(to).map_err(|_| XcmError::from(Error::AccountIdConversionFailed))?;
		let currency_id = CurrencyIdConvert::convert(asset.clone())
			.ok_or_else(|| XcmError::from(Error::CurrencyIdConversionFailed))?;
		let amount: MultiCurrency::Balance = Match::matches_fungible(asset)
			.ok_or_else(|| XcmError::from(Error::FailedToMatchFungible))?
			.saturated_into();
		MultiCurrency::transfer(currency_id, &from_account, &to_account, amount)
			.map_err(|e| XcmError::FailedToTransactAsset(e.into()))?;

		Ok(asset.clone().into())
	}
}
