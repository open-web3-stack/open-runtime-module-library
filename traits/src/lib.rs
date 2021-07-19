#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use impl_trait_for_tuples::impl_for_tuples;
use sp_runtime::{DispatchResult, RuntimeDebug};
use sp_std::{
	cmp::{Eq, PartialEq},
	prelude::Vec,
};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

pub use auction::{Auction, AuctionHandler, AuctionInfo, OnNewBidResult};
pub use currency::{
	BalanceStatus, BasicCurrency, BasicCurrencyExtended, BasicLockableCurrency, BasicReservableCurrency,
	LockIdentifier, MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency, MultiReservableCurrency, OnDust,
};
pub use data_provider::{DataFeeder, DataProvider, DataProviderExtended};
pub use get_by_key::GetByKey;
pub use nft::NFT;
pub use price::{DefaultPriceProvider, PriceProvider};
pub use rewards::RewardHandler;
pub use xcm_transfer::XcmTransfer;

pub mod arithmetic;
pub mod auction;
pub mod currency;
pub mod data_provider;
pub mod get_by_key;
pub mod location;
pub mod nft;
pub mod price;
pub mod rewards;
pub mod xcm_transfer;

/// New data handler
#[impl_trait_for_tuples::impl_for_tuples(30)]
pub trait OnNewData<AccountId, Key, Value> {
	/// New data is available
	fn on_new_data(who: &AccountId, key: &Key, value: &Value);
}

/// Combine data provided by operators
pub trait CombineData<Key, TimestampedValue, ExpiresAt> {
	/// Combine data provided by operators. Optionally includes an expiration
	/// timestamp in the return value
	fn combine_data(
		key: &Key,
		values: Vec<TimestampedValue>,
		prev_value: Option<TimestampedValue>,
	) -> AggregateResult<TimestampedValue, ExpiresAt>;
}

pub enum AggregateResult<TimestampedValue, ExpiresAt> {
	/// get() will return None forever (unless a new value is fed)
	PermanentlyNone,
	/// get() will return None until the given expiration time, after which
	/// the aggregate is reevaluated
	TemporarilyNone(ExpiresAt),
	/// get() will return the given value forever (unless a new value is fed)
	PermanentValue(TimestampedValue),
	/// get() will return the given value until the given expiration time, after
	/// which the aggregate is reevaluated
	TemporaryValue(TimestampedValue, ExpiresAt),
}

impl<TimestampedValue, ExpiresAt> AggregateResult<TimestampedValue, ExpiresAt> {
	pub fn get_value(self) -> Option<TimestampedValue> {
		match self {
			AggregateResult::PermanentValue(value) | AggregateResult::TemporaryValue(value, _) => Some(value),
			_ => None,
		}
	}
}

/// Indicate if should change a value
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub enum Change<Value> {
	/// No change.
	NoChange,
	/// Changed to new value.
	NewValue(Value),
}

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TimestampedValue<Value: Ord + PartialOrd, Moment> {
	pub value: Value,
	pub timestamp: Moment,
}

#[impl_for_tuples(30)]
pub trait Happened<T> {
	fn happened(t: &T);
}

pub trait Handler<T> {
	fn handle(t: &T) -> DispatchResult;
}

#[impl_for_tuples(30)]
impl<T> Handler<T> for Tuple {
	fn handle(t: &T) -> DispatchResult {
		for_tuples!( #( Tuple::handle(t); )* );
		Ok(())
	}
}
