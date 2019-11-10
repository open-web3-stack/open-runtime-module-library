#![cfg_attr(not(feature = "std"), no_std)]

mod default_combine_data;
mod mock;
mod operator_provider;
mod tests;
mod timestamped_value;

pub use default_combine_data::DefaultCombineData;
pub use operator_provider::OperatorProvider;
use rstd::prelude::Vec;
use rstd::result;
use sr_primitives::traits::Member;
use support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Time, Parameter};
use system::ensure_signed;
pub use timestamped_value::TimestampedValue;
pub use traits::{CombineData, OnNewData};

type MomentOf<T> = <<T as Trait>::Time as Time>::Moment;

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type OnNewData: OnNewData<Self::Key, Self::Value>;
	type OperatorProvider: OperatorProvider<Self::AccountId>;
	type CombineData: CombineData<Self::Key, TimestampedValue<Self::Value, MomentOf<Self>>>;
	type Time: Time;
	type Key: Parameter + Member + Copy;
	type Value: Parameter + Member + Copy;
}

decl_storage! {
	trait Store for Module<T: Trait> as Oracle {
		pub RawValues get(raw_values): map (T::AccountId, T::Key) => Option<TimestampedValue<T::Value, MomentOf<T>>>;
		pub HasUpdate get(has_update): map T::Key => bool;
		pub Values get(values): map T::Key => Option<TimestampedValue<T::Value, MomentOf<T>>>;
	}
}

decl_error! {
	// Oracle module errors
	pub enum Error {
		NotSigned,
		NoPermission,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error;
		fn deposit_event() = default;

		pub fn feed_data(origin, key: T::Key, value: T::Value) -> result::Result<(), Error> {
			Self::_feed_data(origin, key, value)
		}
	}
}

decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId,
		<T as Trait>::Key,
		<T as Trait>::Value,
	{
		NewFeedData(AccountId, Key, Value),
	}
);

impl<T: Trait> Module<T> {
	pub fn read_raw_values(key: &T::Key) -> Vec<TimestampedValue<T::Value, MomentOf<T>>> {
		T::OperatorProvider::operators()
			.iter()
			.filter_map(|x| <RawValues<T>>::get((x, *key)))
			.collect()
	}

	pub fn get(key: &T::Key) -> Option<TimestampedValue<T::Value, MomentOf<T>>> {
		if <HasUpdate<T>>::exists(key) {
			let values = Self::read_raw_values(key);
			let result = T::CombineData::combine_data(key, values, None);
			match result {
				Some(timestamped) => {
					<Values<T>>::insert(key, timestamped.clone());
					<HasUpdate<T>>::insert(key, false);
					return Some(timestamped);
				}
				None => return None,
			}
		}
		<Values<T>>::get(key)
	}
}

impl<T: Trait> Module<T> {
	fn _feed_data(origin: T::Origin, key: T::Key, value: T::Value) -> result::Result<(), Error> {
		let who = ensure_signed(origin).map_err(|_| Error::NotSigned)?;
		ensure!(T::OperatorProvider::can_feed_data(&who), Error::NoPermission);

		let timestamp = TimestampedValue {
			value,
			timestamp: T::Time::now(),
		};
		<RawValues<T>>::insert((&who, &key), timestamp);
		<HasUpdate<T>>::insert(&key, true);

		T::OnNewData::on_new_data(&key, &value);

		Self::deposit_event(RawEvent::NewFeedData(who, key, value));
		Ok(())
	}
}
