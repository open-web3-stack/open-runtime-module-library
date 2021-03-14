#![cfg_attr(not(feature = "std"), no_std)]

use codec::FullCodec;
use sp_runtime::traits::{CheckedConversion, Convert, MaybeSerializeDeserialize, SaturatedConversion};
use sp_std::{
	cmp::{Eq, PartialEq},
	collections::btree_set::BTreeSet,
	convert::{TryFrom, TryInto},
	fmt::Debug,
	marker::PhantomData,
	prelude::*,
	result,
};

use xcm::v0::{Error, Junction, MultiAsset, MultiLocation, Result, Xcm};
use xcm_executor::traits::{FilterAssetLocation, LocationConversion, MatchesFungible, NativeAsset, TransactAsset};

use frame_support::{dispatch::DispatchResult, log, traits::Get};

pub trait XcmHandler<AccountId> {
	fn execute_xcm(origin: AccountId, xcm: Xcm) -> DispatchResult;
}

pub trait CurrencyIdConversion<CurrencyId> {
	fn from_asset(asset: &MultiAsset) -> Option<CurrencyId>;
}

pub struct MultiCurrencyAdapter<MultiCurrency, Matcher, AccountIdConverter, AccountId, CurrencyIdConverter, CurrencyId>(
	PhantomData<(
		MultiCurrency,
		Matcher,
		AccountIdConverter,
		AccountId,
		CurrencyIdConverter,
		CurrencyId,
	)>,
);

impl<
		MultiCurrency: orml_traits::MultiCurrency<AccountId, CurrencyId = CurrencyId>,
		Matcher: MatchesFungible<MultiCurrency::Balance>,
		AccountIdConverter: LocationConversion<AccountId>,
		AccountId: sp_std::fmt::Debug,
		CurrencyIdConverter: CurrencyIdConversion<CurrencyId>,
		CurrencyId: FullCodec + Eq + PartialEq + Copy + MaybeSerializeDeserialize + Debug,
	> TransactAsset
	for MultiCurrencyAdapter<MultiCurrency, Matcher, AccountIdConverter, AccountId, CurrencyIdConverter, CurrencyId>
{
	fn deposit_asset(asset: &MultiAsset, location: &MultiLocation) -> Result {
		log::info!("------------------------------------------------");
		log::info!(">>> trying deposit. asset: {:?}, location: {:?}", asset, location);
		let who = AccountIdConverter::from_location(location).ok_or(())?;
		log::info!("who: {:?}", who);
		let currency_id = CurrencyIdConverter::from_asset(asset).ok_or(())?;
		log::info!("currency_id: {:?}", currency_id);
		let amount: MultiCurrency::Balance = Matcher::matches_fungible(&asset).ok_or(())?.saturated_into();
		log::info!("amount: {:?}", amount);
		MultiCurrency::deposit(currency_id, &who, amount).map_err(|_| ())?;
		log::info!(">>> success deposit.");
		log::info!("------------------------------------------------");
		Ok(())
	}

	fn withdraw_asset(asset: &MultiAsset, location: &MultiLocation) -> result::Result<MultiAsset, Error> {
		log::info!("------------------------------------------------");
		log::info!(">>> trying withdraw. asset: {:?}, location: {:?}", asset, location);
		let who = AccountIdConverter::from_location(location).ok_or(())?;
		log::info!("who: {:?}", who);
		let currency_id = CurrencyIdConverter::from_asset(asset).ok_or(())?;
		log::info!("currency_id: {:?}", currency_id);
		let amount: MultiCurrency::Balance = Matcher::matches_fungible(&asset).ok_or(())?.saturated_into();
		log::info!("amount: {:?}", amount);
		MultiCurrency::withdraw(currency_id, &who, amount).map_err(|_| ())?;
		log::info!(">>> success withdraw.");
		log::info!("------------------------------------------------");
		Ok(asset.clone())
	}
}

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
