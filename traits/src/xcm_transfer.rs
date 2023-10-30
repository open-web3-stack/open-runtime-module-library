use sp_runtime::DispatchError;
use sp_std::vec::Vec;
use xcm::{
	v3::{prelude::*, Weight},
	VersionedMultiAsset, VersionedMultiAssets, VersionedMultiLocation,
};
pub struct Transferred<AccountId> {
	pub sender: AccountId,
	pub assets: MultiAssets,
	pub fee: MultiAsset,
	pub dest: MultiLocation,
}

/// Abstraction over cross-chain token transfers.
pub trait XcmTransfer<AccountId, Balance, CurrencyId> {
	/// Transfer local assets with given `CurrencyId` and `Amount`.
	fn transfer(
		who: AccountId,
		currency_id: CurrencyId,
		amount: Balance,
		dest: MultiLocation,
		dest_weight_limit: WeightLimit,
	) -> Result<Transferred<AccountId>, DispatchError>;

	/// Transfer `MultiAsset` assets.
	fn transfer_multiasset(
		who: AccountId,
		asset: MultiAsset,
		dest: MultiLocation,
		dest_weight_limit: WeightLimit,
	) -> Result<Transferred<AccountId>, DispatchError>;

	/// Transfer native currencies specifying the fee and amount as separate.
	fn transfer_with_fee(
		who: AccountId,
		currency_id: CurrencyId,
		amount: Balance,
		fee: Balance,
		dest: MultiLocation,
		dest_weight_limit: WeightLimit,
	) -> Result<Transferred<AccountId>, DispatchError>;

	/// Transfer `MultiAsset` specifying the fee and amount as separate.
	fn transfer_multiasset_with_fee(
		who: AccountId,
		asset: MultiAsset,
		fee: MultiAsset,
		dest: MultiLocation,
		dest_weight_limit: WeightLimit,
	) -> Result<Transferred<AccountId>, DispatchError>;

	/// Transfer several currencies specifying the item to be used as fee.
	fn transfer_multicurrencies(
		who: AccountId,
		currencies: Vec<(CurrencyId, Balance)>,
		fee_item: u32,
		dest: MultiLocation,
		dest_weight_limit: WeightLimit,
	) -> Result<Transferred<AccountId>, DispatchError>;

	/// Transfer several `MultiAsset` specifying the item to be used as fee.
	fn transfer_multiassets(
		who: AccountId,
		assets: MultiAssets,
		fee: MultiAsset,
		dest: MultiLocation,
		dest_weight_limit: WeightLimit,
	) -> Result<Transferred<AccountId>, DispatchError>;
}

pub trait XtokensWeightInfo<AccountId, Balance, CurrencyId> {
	fn weight_of_transfer_multiasset(asset: &VersionedMultiAsset, dest: &VersionedMultiLocation) -> Weight;
	fn weight_of_transfer(currency_id: CurrencyId, amount: Balance, dest: &VersionedMultiLocation) -> Weight;
	fn weight_of_transfer_multicurrencies(
		currencies: &[(CurrencyId, Balance)],
		fee_item: &u32,
		dest: &VersionedMultiLocation,
	) -> Weight;
	fn weight_of_transfer_multiassets(
		assets: &VersionedMultiAssets,
		fee_item: &u32,
		dest: &VersionedMultiLocation,
	) -> Weight;
}
