#![cfg_attr(not(feature = "std"), no_std)]

use codec::FullCodec;
use sp_std::{marker::PhantomData, result, cmp::{Eq, PartialEq}, convert::TryInto, fmt::Debug};
use sp_runtime::traits::{MaybeSerializeDeserialize, SaturatedConversion};
use xcm::v0::{MultiAsset, MultiLocation, Error, Result};
use xcm_executor::traits::{MatchesFungible, LocationConversion, TransactAsset};

pub trait CurrencyIdConversion<CurrencyId> {
	fn from_asset(asset: &MultiAsset) -> Option<CurrencyId>;
}

pub struct MultiCurrencyAdapter<MultiCurrency, Matcher, AccountIdConverter, AccountId, CurrencyIdConverter, CurrencyId>(
	PhantomData<MultiCurrency>,
	PhantomData<Matcher>,
	PhantomData<AccountIdConverter>,
	PhantomData<AccountId>,
	PhantomData<CurrencyIdConverter>,
	PhantomData<CurrencyId>,
);

impl<
	MultiCurrency: orml_traits::MultiCurrency<AccountId, CurrencyId=CurrencyId>,
	Matcher: MatchesFungible<MultiCurrency::Balance>,
	AccountIdConverter: LocationConversion<AccountId>,
	AccountId,
	CurrencyIdConverter: CurrencyIdConversion<CurrencyId>,
	CurrencyId: FullCodec + Eq + PartialEq + Copy + MaybeSerializeDeserialize + Debug,
> TransactAsset for MultiCurrencyAdapter<MultiCurrency, Matcher, AccountIdConverter, AccountId, CurrencyIdConverter, CurrencyId> {
	fn deposit_asset(asset: &MultiAsset, location: &MultiLocation) -> Result {
		let who = AccountIdConverter::from_location(location).ok_or(())?;
		let currency_id = CurrencyIdConverter::from_asset(asset).ok_or(())?;
		let amount = Matcher::matches_fungible(&asset).ok_or(())?.saturated_into();
		let balance_amount = amount.try_into().map_err(|_| ())?;
		MultiCurrency::deposit(currency_id, &who, balance_amount).map_err( |_| ())?;
		Ok(())
	}

	fn withdraw_asset(asset: &MultiAsset, location: &MultiLocation) -> result::Result<MultiAsset, Error> {
		let who = AccountIdConverter::from_location(location).ok_or(())?;
		let currency_id = CurrencyIdConverter::from_asset(asset).ok_or(())?;
		let amount = Matcher::matches_fungible(&asset).ok_or(())?.saturated_into();
		let balance_amount = amount.try_into().map_err(|_| ())?;
		MultiCurrency::withdraw(currency_id, &who, balance_amount).map_err(|_| ())?;
		Ok(asset.clone())
	}
}
