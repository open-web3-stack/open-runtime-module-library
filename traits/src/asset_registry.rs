use frame_support::pallet_prelude::*;
use sp_runtime::DispatchResult;
use sp_std::fmt::Debug;
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
#[derive(TypeInfo, Encode, Decode, CloneNoBound, EqNoBound, PartialEqNoBound, RuntimeDebugNoBound, MaxEncodedLen)]
#[codec(mel_bound(skip_type_params(StringLimit)))]
#[scale_info(skip_type_params(StringLimit))]

pub struct AssetMetadata<Balance, CustomMetadata, StringLimit: Get<u32>>
where
	Balance: Clone + Debug + Eq + PartialEq,
	CustomMetadata: Parameter + Member + TypeInfo,
{
	pub decimals: u32,
	pub name: BoundedVec<u8, StringLimit>,
	pub symbol: BoundedVec<u8, StringLimit>,
	pub existential_deposit: Balance,
	pub location: Option<VersionedMultiLocation>,
	pub additional: CustomMetadata,
}

pub trait Inspect {
	/// AssetId type
	type AssetId;
	/// Balance type
	type Balance: Clone + Debug + Eq + PartialEq;
	/// Custom metadata type
	type CustomMetadata: Parameter + Member + TypeInfo;
	/// Name and symbol string limit
	type StringLimit: Get<u32>;

	fn asset_id(location: &MultiLocation) -> Option<Self::AssetId>;
	fn metadata(
		asset_id: &Self::AssetId,
	) -> Option<AssetMetadata<Self::Balance, Self::CustomMetadata, Self::StringLimit>>;
	fn metadata_by_location(
		location: &MultiLocation,
	) -> Option<AssetMetadata<Self::Balance, Self::CustomMetadata, Self::StringLimit>>;
	fn location(asset_id: &Self::AssetId) -> Result<Option<MultiLocation>, DispatchError>;
}

pub trait Mutate: Inspect {
	fn register_asset(
		asset_id: Option<Self::AssetId>,
		metadata: AssetMetadata<Self::Balance, Self::CustomMetadata, Self::StringLimit>,
	) -> DispatchResult;

	fn update_asset(
		asset_id: Self::AssetId,
		decimals: Option<u32>,
		name: Option<BoundedVec<u8, Self::StringLimit>>,
		symbol: Option<BoundedVec<u8, Self::StringLimit>>,
		existential_deposit: Option<Self::Balance>,
		location: Option<Option<VersionedMultiLocation>>,
		additional: Option<Self::CustomMetadata>,
	) -> DispatchResult;
}
