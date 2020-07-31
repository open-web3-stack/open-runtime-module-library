#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use sp_runtime::{DispatchResult, RuntimeDebug};
use sp_std::{
	cmp::{Eq, PartialEq},
	prelude::Vec,
};

pub use auction::{Auction, AuctionHandler, AuctionInfo, OnNewBidResult};
pub use currency::{
	BalanceStatus, BasicCurrency, BasicCurrencyExtended, BasicLockableCurrency, BasicReservableCurrency,
	LockIdentifier, MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency, MultiReservableCurrency, OnReceived,
};
pub use price::{DefaultPriceProvider, PriceProvider};

pub mod arithmetic;
pub mod auction;
pub mod currency;
pub mod price;

/// New data handler
#[impl_trait_for_tuples::impl_for_tuples(30)]
pub trait OnNewData<AccountId, Key, Value> {
	/// New data is available
	fn on_new_data(who: &AccountId, key: &Key, value: &Value);
}

/// A simple trait to provide data
pub trait DataProvider<Key, Value> {
	/// Get data by key
	fn get(key: &Key) -> Option<Value>;
}

/// Data provider with ability to insert data
pub trait DataProviderExtended<Key, Value, AccountId>: DataProvider<Key, Value> {
	/// Provide a new value for a given key from an operator
	fn feed_value(who: AccountId, key: Key, value: Value) -> DispatchResult;
}

/// Combine data provided by operators
pub trait CombineData<Key, TimestampedValue> {
	/// Combine data provided by operators
	fn combine_data(
		key: &Key,
		values: Vec<TimestampedValue>,
		prev_value: Option<TimestampedValue>,
	) -> Option<TimestampedValue>;
}

/// Indicate if should change a value
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub enum Change<Value> {
	/// No change.
	NoChange,
	/// Changed to new value.
	NewValue(Value),
}
