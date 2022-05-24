use frame_support::pallet_prelude::*;
use xcm::latest::prelude::*;

pub trait WeightToFeeConverter {
	fn convert_weight_to_fee(location: &MultiLocation, weight: Weight) -> Option<u128>;
}

pub trait FixedConversionRateProvider {
	fn get_fee_per_second(location: &MultiLocation) -> Option<u128>;
}

pub trait AssetProcessor<AssetId, Metadata> {
	fn pre_register(id: Option<AssetId>, asset_metadata: Metadata) -> Result<(AssetId, Metadata), DispatchError>;
	fn post_register(_id: AssetId, _asset_metadata: Metadata) -> Result<(), DispatchError> {
		Ok(())
	}
}
