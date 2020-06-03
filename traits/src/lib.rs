#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use sp_runtime::{DispatchResult, RuntimeDebug};
use sp_std::{
	cmp::{Eq, PartialEq},
	prelude::Vec,
};

pub use auction::{Auction, AuctionEndChange, AuctionHandler, AuctionInfo, OnNewBidResult};
pub use currency::{
	BalanceStatus, BasicCurrency, BasicCurrencyExtended, BasicLockableCurrency, BasicReservableCurrency,
	LockIdentifier, MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency, MultiReservableCurrency,
	OnDustRemoval, OnReceived,
};

pub mod arithmetic;
pub mod auction;
pub mod currency;

#[impl_trait_for_tuples::impl_for_tuples(30)]
pub trait OnNewData<AccountId, Key, Value> {
	fn on_new_data(who: &AccountId, key: &Key, value: &Value);
}

pub trait DataProvider<Key, Value> {
	fn get(key: &Key) -> Option<Value>;
}

pub trait DataProviderExtended<Key, Value, AccountId>: DataProvider<Key, Value> {
	fn feed_value(who: AccountId, key: Key, value: Value) -> DispatchResult;
}

pub trait PriceProvider<CurrencyId, Price> {
	fn get_price(base: CurrencyId, quote: CurrencyId) -> Option<Price>;
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

#[impl_trait_for_tuples::impl_for_tuples(30)]
pub trait OnRedundantCall<AccountId> {
	fn multiple_calls_per_block(who: &AccountId);
}

#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug)]
pub enum DelayedDispatchTime<BlockNumber> {
	At(BlockNumber),
	After(BlockNumber),
}

pub type DispatchId = u32;

pub trait Scheduler<BlockNumber> {
	type Origin;
	type Call;

	fn schedule(origin: Self::Origin, call: Self::Call, when: DelayedDispatchTime<BlockNumber>) -> DispatchId;
	fn cancel(id: DispatchId);
}
