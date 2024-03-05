use sp_runtime::DispatchError;
use sp_std::vec::Vec;
use xcm::{
	v4::{prelude::*, Weight},
	VersionedAsset, VersionedAssets, VersionedLocation,
};
pub struct Transferred<AccountId> {
	pub sender: AccountId,
	pub assets: Assets,
	pub fee: Asset,
	pub dest: Location,
}

/// Abstraction over cross-chain token transfers.
pub trait XcmTransfer<AccountId, Balance, CurrencyId> {
	/// Transfer local assets with given `CurrencyId` and `Amount`.
	fn transfer(
		who: AccountId,
		currency_id: CurrencyId,
		amount: Balance,
		dest: Location,
		dest_weight_limit: WeightLimit,
	) -> Result<Transferred<AccountId>, DispatchError>;

	/// Transfer `Asset` assets.
	fn transfer_multiasset(
		who: AccountId,
		asset: Asset,
		dest: Location,
		dest_weight_limit: WeightLimit,
	) -> Result<Transferred<AccountId>, DispatchError>;

	/// Transfer native currencies specifying the fee and amount as separate.
	fn transfer_with_fee(
		who: AccountId,
		currency_id: CurrencyId,
		amount: Balance,
		fee: Balance,
		dest: Location,
		dest_weight_limit: WeightLimit,
	) -> Result<Transferred<AccountId>, DispatchError>;

	/// Transfer `Asset` specifying the fee and amount as separate.
	fn transfer_multiasset_with_fee(
		who: AccountId,
		asset: Asset,
		fee: Asset,
		dest: Location,
		dest_weight_limit: WeightLimit,
	) -> Result<Transferred<AccountId>, DispatchError>;

	/// Transfer several currencies specifying the item to be used as fee.
	fn transfer_multicurrencies(
		who: AccountId,
		currencies: Vec<(CurrencyId, Balance)>,
		fee_item: u32,
		dest: Location,
		dest_weight_limit: WeightLimit,
	) -> Result<Transferred<AccountId>, DispatchError>;

	/// Transfer several `Asset` specifying the item to be used as fee.
	fn transfer_multiassets(
		who: AccountId,
		assets: Assets,
		fee: Asset,
		dest: Location,
		dest_weight_limit: WeightLimit,
	) -> Result<Transferred<AccountId>, DispatchError>;
}

pub trait XtokensWeightInfo<AccountId, Balance, CurrencyId> {
	fn weight_of_transfer_multiasset(asset: &VersionedAsset, dest: &VersionedLocation) -> Weight;
	fn weight_of_transfer(currency_id: CurrencyId, amount: Balance, dest: &VersionedLocation) -> Weight;
	fn weight_of_transfer_multicurrencies(
		currencies: &[(CurrencyId, Balance)],
		fee_item: &u32,
		dest: &VersionedLocation,
	) -> Weight;
	fn weight_of_transfer_multiassets(assets: &VersionedAssets, fee_item: &u32, dest: &VersionedLocation) -> Weight;
}
