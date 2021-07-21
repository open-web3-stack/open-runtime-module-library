use frame_support::dispatch::DispatchError;
use frame_support::weights::Weight;
use xcm::opaque::v0::{MultiAsset, MultiLocation, Outcome};

pub type XcmExecutionResult = sp_std::result::Result<Outcome, DispatchError>;

/// Abstraction over cross-chain token transfers.
pub trait XcmTransfer<AccountId, Balance, CurrencyId> {
	/// Transfer native currencies.
	fn transfer(
		who: AccountId,
		currency_id: CurrencyId,
		amount: Balance,
		dest: MultiLocation,
		dest_weight: Weight,
	) -> XcmExecutionResult;

	/// Transfer `MultiAsset`
	fn transfer_multi_asset(
		who: AccountId,
		asset: MultiAsset,
		dest: MultiLocation,
		dest_weight: Weight,
	) -> XcmExecutionResult;
}
