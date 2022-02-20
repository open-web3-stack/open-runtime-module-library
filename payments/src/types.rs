#![allow(unused_qualifications)]
use crate::{pallet, AssetIdOf, BalanceOf, BoundedDataOf};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{DispatchResult, Percent};

/// The PaymentDetail struct stores information about the payment/escrow
/// A "payment" in virto network is similar to an escrow, it is used to
/// guarantee proof of funds and can be released once an agreed upon condition
/// has reached between the payment creator and recipient. The payment lifecycle
/// is tracked using the state field.
#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound(T: pallet::Config))]
pub struct PaymentDetail<T: pallet::Config> {
	/// type of asset used for payment
	pub asset: AssetIdOf<T>,
	/// amount of asset used for payment
	pub amount: BalanceOf<T>,
	/// incentive amount that is credited to creator for resolving
	pub incentive_amount: BalanceOf<T>,
	/// enum to track payment lifecycle [Created, NeedsReview, RefundRequested,
	/// Requested]
	pub state: PaymentState<T::BlockNumber>,
	/// account that can settle any disputes created in the payment
	pub resolver_account: T::AccountId,
	/// fee charged and recipient account details
	pub fee_detail: Option<(T::AccountId, BalanceOf<T>)>,
	/// remarks to give context to payment
	pub remark: Option<BoundedDataOf<T>>,
}

/// The `PaymentState` enum tracks the possible states that a payment can be in.
/// When a payment is 'completed' or 'cancelled' it is removed from storage and
/// hence not tracked by a state.
#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, MaxEncodedLen, TypeInfo)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PaymentState<BlockNumber> {
	/// Amounts have been reserved and waiting for release/cancel
	Created,
	/// A judge needs to review and release manually
	NeedsReview,
	/// The user has requested refund and will be processed by `BlockNumber`
	RefundRequested(BlockNumber),
	/// The recipient of this transaction has created a request
	PaymentRequested,
}

/// trait that defines how to create/release payments for users
pub trait PaymentHandler<T: pallet::Config> {
	/// Create a PaymentDetail from the given payment details
	/// Calculate the fee amount and store PaymentDetail in storage
	/// Possible reasons for failure include:
	/// - Payment already exists and cannot be overwritten
	fn create_payment(
		from: T::AccountId,
		to: T::AccountId,
		asset: AssetIdOf<T>,
		amount: BalanceOf<T>,
		payment_state: PaymentState<T::BlockNumber>,
		incentive_percentage: Percent,
		remark: Option<BoundedDataOf<T>>,
	) -> Result<PaymentDetail<T>, sp_runtime::DispatchError>;

	/// Attempt to reserve an amount of the given asset from the caller
	/// If not possible then return Error. Possible reasons for failure include:
	/// - User does not have enough balance.
	fn reserve_payment_amount(from: &T::AccountId, to: &T::AccountId, payment: PaymentDetail<T>) -> DispatchResult;

	// Settle a payment of `from` to `to`. To release a payment, the
	// recipient_share=100, to cancel a payment recipient_share=0
	// Possible reasonse for failure include
	/// - The payment does not exist
	/// - The unreserve operation fails
	/// - The transfer operation fails
	fn settle_payment(from: T::AccountId, to: T::AccountId, recipient_share: Percent) -> DispatchResult;

	/// Attempt to fetch the details of a payment from the given payment_id
	/// Possible reasons for failure include:
	/// - The payment does not exist
	fn get_payment_details(from: T::AccountId, to: T::AccountId) -> Option<PaymentDetail<T>>;
}

/// DisputeResolver trait defines how to create/assing judges for solving
/// payment disputes
pub trait DisputeResolver<Account> {
	/// Get a DisputeResolver (Judge) account
	fn get_origin() -> Account;
}

/// Fee Handler trait that defines how to handle marketplace fees to every
/// payment/swap
pub trait FeeHandler<T: pallet::Config> {
	/// Get the distribution of fees to marketplace participants
	fn apply_fees(from: &T::AccountId, to: &T::AccountId, detail: &PaymentDetail<T>) -> (T::AccountId, Percent);
}
