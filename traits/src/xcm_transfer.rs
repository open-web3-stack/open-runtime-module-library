use frame_support::dispatch::DispatchResult;
use xcm::latest::prelude::*;

/// Abstraction over cross-chain token transfers.
pub trait XcmTransfer<AccountId, Balance, CurrencyId> {
	/// Transfer local assets with given `CurrencyId` and `Amount`.
	fn transfer(
		who: AccountId,
		currency_id: CurrencyId,
		amount: Balance,
		dest: MultiLocation,
		dest_weight_limit: WeightLimit,
	) -> DispatchResult;

	/// Transfer `MultiAsset` assets.
	fn transfer_multiasset(
		who: AccountId,
		asset: MultiAsset,
		dest: MultiLocation,
		dest_weight_limit: WeightLimit,
	) -> DispatchResult;

	/// Transfer native currencies specifying the fee and amount as separate.
	fn transfer_with_fee(
		who: AccountId,
		currency_id: CurrencyId,
		amount: Balance,
		fee: Balance,
		dest: MultiLocation,
		dest_weight_limit: WeightLimit,
	) -> DispatchResult;

	/// Transfer `MultiAsset` specifying the fee and amount as separate.
	fn transfer_multiasset_with_fee(
		who: AccountId,
		asset: MultiAsset,
		fee: MultiAsset,
		dest: MultiLocation,
		dest_weight_limit: WeightLimit,
	) -> DispatchResult;

	/// Transfer several currencies specifying the item to be used as fee.
	fn transfer_multicurrencies(
		who: AccountId,
		currencies: Vec<(CurrencyId, Balance)>,
		fee_item: u32,
		dest: MultiLocation,
		dest_weight_limit: WeightLimit,
	) -> DispatchResult;

	/// Transfer several `MultiAsset` specifying the item to be used as fee.
	fn transfer_multiassets(
		who: AccountId,
		assets: MultiAssets,
		fee: MultiAsset,
		dest: MultiLocation,
		dest_weight_limit: WeightLimit,
	) -> DispatchResult;
}
