use frame_support::pallet_prelude::*;
use sp_runtime::DispatchResult;
use sp_std::vec::Vec;
use xcm::v3::prelude::*;
use xcm::VersionedMultiLocation;

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

/// Data describing the asset properties.
#[derive(scale_info::TypeInfo, Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct AssetMetadata<Balance, CustomMetadata: Parameter + Member + TypeInfo> {
	pub decimals: u32,
	pub name: Vec<u8>,
	pub symbol: Vec<u8>,
	pub existential_deposit: Balance,
	pub location: Option<VersionedMultiLocation>,
	pub additional: CustomMetadata,
}

pub trait Inspect {
	/// AssetId type
	type AssetId;
	/// Balance type
	type Balance;
	/// Custom metadata type
	type CustomMetadata: Parameter + Member + TypeInfo;

	fn asset_id(location: &MultiLocation) -> Option<Self::AssetId>;
	fn metadata(asset_id: &Self::AssetId) -> Option<AssetMetadata<Self::Balance, Self::CustomMetadata>>;
	fn metadata_by_location(location: &MultiLocation) -> Option<AssetMetadata<Self::Balance, Self::CustomMetadata>>;
	fn location(asset_id: &Self::AssetId) -> Result<Option<MultiLocation>, DispatchError>;
}

pub trait Mutate: Inspect {
	fn register_asset(
		asset_id: Option<Self::AssetId>,
		metadata: AssetMetadata<Self::Balance, Self::CustomMetadata>,
	) -> DispatchResult;

	fn update_asset(
		asset_id: Self::AssetId,
		decimals: Option<u32>,
		name: Option<Vec<u8>>,
		symbol: Option<Vec<u8>>,
		existential_deposit: Option<Self::Balance>,
		location: Option<Option<VersionedMultiLocation>>,
		additional: Option<Self::CustomMetadata>,
	) -> DispatchResult;
}
