#![cfg_attr(not(feature = "std"), no_std)]

use codec::FullCodec;
use sp_std::{prelude::*, marker::PhantomData, result, cmp::{Eq, PartialEq}, convert::{TryInto, TryFrom}, fmt::Debug};
use sp_runtime::{traits::{MaybeSerializeDeserialize, CheckedConversion, SaturatedConversion}, DispatchResult};

use xcm::v0::{MultiAsset, MultiLocation, Error, Result, Junction};
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

pub trait XcmHandler {
	type Origin;
	type Xcm;
	fn execute(origin: Self::Origin, xcm: Self::Xcm) -> DispatchResult;
}

pub struct IsConcreteWithGeneralKey<CurrencyId>(PhantomData<CurrencyId>);
impl<CurrencyId: TryFrom<Vec<u8>>, B: TryFrom<u128>> MatchesFungible<B> for IsConcreteWithGeneralKey<CurrencyId> {
	fn matches_fungible(a: &MultiAsset) -> Option<B> {
		if let MultiAsset::ConcreteFungible { id, amount } = a {
			if let MultiLocation::X1(Junction::GeneralKey(key)) = id {
				if TryInto::<CurrencyId>::try_into(key.clone()).is_ok() {
					return CheckedConversion::checked_from(*amount);
				}
			}
		}
		None
	}
}
