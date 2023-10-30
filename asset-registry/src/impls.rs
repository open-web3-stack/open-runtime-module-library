use crate::module::*;
use frame_support::{pallet_prelude::*, weights::constants::WEIGHT_REF_TIME_PER_SECOND};
use orml_traits::{
	asset_registry::{
		AssetMetadata, AssetProcessor, FixedConversionRateProvider, Inspect, Mutate, WeightToFeeConverter,
	},
	GetByKey,
};
use sp_runtime::FixedPointNumber;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, Bounded, CheckedAdd, One},
	ArithmeticError, FixedU128,
};
use sp_std::prelude::*;
use xcm::v3::{prelude::*, Weight as XcmWeight};
use xcm::VersionedMultiLocation;
use xcm_builder::TakeRevenue;
use xcm_executor::{traits::WeightTrader, Assets};

/// Alias for AssetMetadata to improve readability (and to placate clippy)
pub type DefaultAssetMetadata<T> =
	AssetMetadata<<T as Config>::Balance, <T as Config>::CustomMetadata, <T as Config>::StringLimit>;

/// An AssetProcessor that assigns a sequential ID
pub struct SequentialId<T>(PhantomData<T>);

impl<T> AssetProcessor<T::AssetId, DefaultAssetMetadata<T>> for SequentialId<T>
where
	T: Config,
	T::AssetId: AtLeast32BitUnsigned,
{
	fn pre_register(
		id: Option<T::AssetId>,
		asset_metadata: DefaultAssetMetadata<T>,
	) -> Result<(T::AssetId, DefaultAssetMetadata<T>), DispatchError> {
		let next_id = LastAssetId::<T>::get()
			.checked_add(&T::AssetId::one())
			.ok_or(ArithmeticError::Overflow)?;

		match id {
			Some(explicit_id) if explicit_id != next_id => {
				// we don't allow non-sequential ids
				Err(Error::<T>::InvalidAssetId.into())
			}
			_ => {
				LastAssetId::<T>::put(&next_id);
				Ok((next_id, asset_metadata))
			}
		}
	}
}

/// A default implementation for WeightToFeeConverter that takes a fixed
/// conversion rate.
pub struct FixedRateAssetRegistryTrader<P: FixedConversionRateProvider>(PhantomData<P>);
impl<P: FixedConversionRateProvider> WeightToFeeConverter for FixedRateAssetRegistryTrader<P> {
	fn convert_weight_to_fee(location: &MultiLocation, weight: Weight) -> Option<u128> {
		let fee_per_second = P::get_fee_per_second(location)?;
		let weight_ratio = FixedU128::saturating_from_rational(weight.ref_time(), WEIGHT_REF_TIME_PER_SECOND);
		let amount = weight_ratio.saturating_mul_int(fee_per_second);
		Some(amount)
	}
}

/// Helper struct for the AssetRegistryTrader that stores the data about
/// bought weight.
pub struct BoughtWeight {
	weight: Weight,
	asset_location: MultiLocation,
	amount: u128,
}

/// A WeightTrader implementation that tries to buy weight using a single
/// currency. It tries all assets in `payment` and uses the first asset that can
/// cover the weight. This asset is then "locked in" - later calls to
/// `buy_weight` in the same xcm message only try the same asset.
/// This is because only a single asset can be refunded due to the return type
/// of `refund_weight`. This implementation assumes that `WeightToFeeConverter`
/// implements a linear function, i.e. fee(x) + fee(y) = fee(x+y).
pub struct AssetRegistryTrader<W: WeightToFeeConverter, R: TakeRevenue> {
	bought_weight: Option<BoughtWeight>,
	_phantom: PhantomData<(W, R)>,
}

impl<W: WeightToFeeConverter, R: TakeRevenue> WeightTrader for AssetRegistryTrader<W, R> {
	fn new() -> Self {
		Self {
			bought_weight: None,
			_phantom: Default::default(),
		}
	}

	fn buy_weight(&mut self, weight: XcmWeight, payment: Assets, _context: &XcmContext) -> Result<Assets, XcmError> {
		log::trace!(
			target: "xcm::weight",
			"AssetRegistryTrader::buy_weight weight: {:?}, payment: {:?}",
			weight, payment,
		);

		for (asset, _) in payment.fungible.iter() {
			if let AssetId::Concrete(ref location) = asset {
				if matches!(self.bought_weight, Some(ref bought) if &bought.asset_location != location) {
					// we already bought another asset - don't attempt to buy this one since
					// we won't be able to refund it
					continue;
				}

				if let Some(fee_increase) = W::convert_weight_to_fee(location, weight) {
					if fee_increase == 0 {
						// if the fee is set very low it could lead to zero fees, in which case
						// constructing the fee asset item to subtract from payment would fail.
						// Therefore, provide early exit
						return Ok(payment);
					}

					if let Ok(unused) = payment.clone().checked_sub((*asset, fee_increase).into()) {
						let (existing_weight, existing_fee) = match self.bought_weight {
							Some(ref x) => (x.weight, x.amount),
							None => (Weight::zero(), 0),
						};

						self.bought_weight = Some(BoughtWeight {
							amount: existing_fee.checked_add(fee_increase).ok_or(XcmError::Overflow)?,
							weight: existing_weight.checked_add(&weight).ok_or(XcmError::Overflow)?,
							asset_location: *location,
						});
						return Ok(unused);
					}
				}
			}
		}
		Err(XcmError::TooExpensive)
	}

	fn refund_weight(&mut self, weight: XcmWeight, _context: &XcmContext) -> Option<MultiAsset> {
		log::trace!(target: "xcm::weight", "AssetRegistryTrader::refund_weight weight: {:?}", weight);

		match self.bought_weight {
			Some(ref mut bought) => {
				let new_weight = bought.weight.saturating_sub(weight);
				let new_amount = W::convert_weight_to_fee(&bought.asset_location, new_weight)?;
				let refunded_amount = bought.amount.saturating_sub(new_amount);

				bought.weight = new_weight;
				bought.amount = new_amount;

				Some((AssetId::Concrete(bought.asset_location), refunded_amount).into())
			}
			None => None, // nothing to refund
		}
	}
}

impl<W: WeightToFeeConverter, R: TakeRevenue> Drop for AssetRegistryTrader<W, R> {
	fn drop(&mut self) {
		if let Some(ref bought) = self.bought_weight {
			R::take_revenue((AssetId::Concrete(bought.asset_location), bought.amount).into());
		}
	}
}

pub struct ExistentialDeposits<T: Config>(PhantomData<T>);

// Return Existential deposit of an asset. Implementing this trait allows the
// pallet to be used in the tokens::ExistentialDeposits config item
impl<T: Config> GetByKey<T::AssetId, T::Balance> for ExistentialDeposits<T> {
	fn get(k: &T::AssetId) -> T::Balance {
		if let Some(metadata) = Pallet::<T>::metadata(k) {
			metadata.existential_deposit
		} else {
			// Asset does not exist - not supported
			T::Balance::max_value()
		}
	}
}

impl<T: Config> Inspect for Pallet<T> {
	type AssetId = T::AssetId;
	type Balance = T::Balance;
	type CustomMetadata = T::CustomMetadata;
	type StringLimit = T::StringLimit;

	fn asset_id(location: &MultiLocation) -> Option<Self::AssetId> {
		Pallet::<T>::location_to_asset_id(location)
	}

	fn metadata(id: &Self::AssetId) -> Option<AssetMetadata<Self::Balance, Self::CustomMetadata, Self::StringLimit>> {
		Pallet::<T>::metadata(id)
	}

	fn metadata_by_location(
		location: &MultiLocation,
	) -> Option<AssetMetadata<Self::Balance, Self::CustomMetadata, Self::StringLimit>> {
		Pallet::<T>::fetch_metadata_by_location(location)
	}

	fn location(asset_id: &Self::AssetId) -> Result<Option<MultiLocation>, DispatchError> {
		Pallet::<T>::multilocation(asset_id)
	}
}

impl<T: Config> Mutate for Pallet<T> {
	fn register_asset(
		asset_id: Option<Self::AssetId>,
		metadata: AssetMetadata<Self::Balance, Self::CustomMetadata, Self::StringLimit>,
	) -> DispatchResult {
		Pallet::<T>::do_register_asset(metadata, asset_id)
	}

	fn update_asset(
		asset_id: Self::AssetId,
		decimals: Option<u32>,
		name: Option<BoundedVec<u8, Self::StringLimit>>,
		symbol: Option<BoundedVec<u8, Self::StringLimit>>,
		existential_deposit: Option<Self::Balance>,
		location: Option<Option<VersionedMultiLocation>>,
		additional: Option<Self::CustomMetadata>,
	) -> DispatchResult {
		Pallet::<T>::do_update_asset(
			asset_id,
			decimals,
			name,
			symbol,
			existential_deposit,
			location,
			additional,
		)
	}
}
