#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use sp_runtime::{DispatchResult, RuntimeDebug};
use sp_std::{
	cmp::{Eq, Ordering, PartialEq},
	prelude::Vec,
};

pub use auction::{Auction, AuctionHandler, AuctionInfo, OnNewBidResult};
pub use currency::{
	BalanceStatus, BasicCurrency, BasicCurrencyExtended, BasicLockableCurrency, BasicReservableCurrency,
	LockIdentifier, MultiCurrency, MultiCurrencyExtended, MultiLockableCurrency, MultiReservableCurrency, OnReceived,
};
pub use price::{DefaultPriceProvider, PriceProvider};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
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

/// A simple trait to provide data for api
pub trait DataProviderExtended<Key, Value> {
	/// Provide a value with timestamp
	fn get_no_op(key: &Key) -> Option<Value>;
	/// Provide a list of tuples of currency and value with timestamp
	fn get_all_values() -> Vec<(Key, Option<Value>)>;
}

/// Data provider with ability to provide data with no-op, and provide all data.
pub trait DataFeeder<Key, Value, AccountId>: DataProvider<Key, Value> {
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

#[derive(Encode, Decode, RuntimeDebug, Eq, PartialEq, Clone, Copy)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TimestampedValue<Value, Moment> {
	pub value: Value,
	pub timestamp: Moment,
}

/// A simple trait to provide data from a given ProviderId
pub trait MultiDataProvider<ProviderId, Key, Value> {
	/// Provide a new value for given key and ProviderId from an operator
	fn get(source: ProviderId, key: &Key) -> Option<Value>;
}

pub fn find_median<T, F, M>(mut data: Vec<T>, compare: F, merge: M) -> Option<T>
where
	F: FnMut(&T, &T) -> Ordering,
	M: Fn(T, T) -> Option<T>,
{
	if data.len() == 0 {
		return None;
	}
	data.sort_by(compare);
	let mid = data.len() / 2;
	// data.len() < 0 will never happen because usize is always >= 0
	// If data.len() == 0 we never reach here (if above)
	// If data.len() == 1, mid will be 0 and we go to "else" branch. not crash.
	// If data.len() >= 2,
	//     mid will always be < data.len() and so mid-1
	//     mid will always be >= 1 and so mid-1 will always be >= 0
	if data.len() % 2 == 0 {
		merge(data.swap_remove(mid), data.swap_remove(mid - 1))
	} else {
		Some(data.swap_remove(mid))
	}
}

#[macro_export]
macro_rules! create_median_value_data_provider {
	(
		$TypeName:ident, $( $Provider:ty ),*
	) => {
		pub struct $TypeName;
		impl DataProvider<CurrencyId, Price> for $TypeName {
			fn get(key: &CurrencyId) -> Option<Price> {
				let mut values: Vec<Price> = Vec::new();
				$(
					match <$Provider as DataProvider<CurrencyId, Price>>::get(&key) {
						Some(value) => values.push(value),
						None => ()
					}
				)*

				find_median(
					values,
					|a, b| a.cmp(&b),
					|a, b| Some((a+b)/FixedU128::saturating_from_integer(2))
				)
			}
		}

		impl DataProviderExtended<CurrencyId, TimestampedValue<Price, Moment>> for $TypeName {
			fn get_no_op(key: &CurrencyId) -> Option<TimestampedValue<Price, Moment>> {
				let mut values: Vec<TimestampedValue<Price, Moment>> = Vec::new();
				$(
					match <$Provider as DataProviderExtended<CurrencyId, TimestampedValue<Price, Moment>>>::get_no_op(&key) {
						Some(value) => values.push(value),
						None => ()
					}
				)*

				find_median(
					values,
					|a, b| a.value.cmp(&b.value),
					|a, b| Some(TimestampedValue {
						value: (a.value + b.value) / FixedU128::saturating_from_integer(2),
						timestamp: (a.timestamp + b.timestamp) / 2u64,
					})
				)
			}

			fn get_all_values() -> Vec<(CurrencyId, Option<TimestampedValue<Price, Moment>>)> {
				let mut temp: Vec<(CurrencyId, Vec<TimestampedValue<Price, Moment>>)> = Vec::new();
				$(
					for (k1, values_opt) in <$Provider as DataProviderExtended<CurrencyId, TimestampedValue<Price, Moment>>>::get_all_values() {
						let mut i = 0;
						let mut found = false;
						for (k2, _) in &temp {
							if k1 == *k2 {
								found = true;
								break;
							}
							i = i + 1;
						}
						match (found, values_opt) {
							(true, Some(v)) => temp[i].1.push(v),
							(true, None) => (),
							(false, Some(v)) => temp.push((k1, vec![v])),
							(false, None) => temp.push((k1, vec![])),
						}
					}
				)*

				temp.iter_mut().map(|(key, values)| (*key, {
					find_median(
						values.to_vec(),
						|a, b| a.value.cmp(&b.value),
						|a, b| Some(TimestampedValue {
							value: (a.value + b.value) / FixedU128::saturating_from_integer(2),
							timestamp: (a.timestamp + b.timestamp) / 2u64,
						})
					)
				})).collect::<Vec<(CurrencyId, Option<TimestampedValue<Price, Moment>>)>>()
			}
		}
	};
}
