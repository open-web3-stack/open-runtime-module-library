use frame_support::dispatch::DispatchResult;
use xcm::latest::prelude::*;

/// Abstraction over cross-chain token transfers.
pub trait XcmTransfer<AccountId, Balance, CurrencyId> {
	/// Transfer native currencies.
	fn transfer(
		who: AccountId,
		currency_id: CurrencyId,
		amount: Balance,
		dest: MultiLocation,
		dest_weight_limit: WeightLimit,
	) -> DispatchResult;

	/// Transfer `MultiAsset`
	fn transfer_multi_asset(
		who: AccountId,
		asset: MultiAsset,
		dest: MultiLocation,
		dest_weight_limit: WeightLimit,
	) -> DispatchResult;
}
