#![cfg_attr(not(feature = "std"), no_std)]

mod default_combine_data;
mod mock;
mod operator_provider;
mod tests;
mod timestamped_value;

pub use default_combine_data::DefaultCombineData;
pub use operator_provider::OperatorProvider;
use rstd::{prelude::Vec, result};
use sr_primitives::traits::Member;
use support::{decl_error, decl_event, decl_module, decl_storage, dispatch::Result, ensure, traits::Time, Parameter};
use system::ensure_signed;
pub use timestamped_value::TimestampedValue;
pub use traits::{CombineData, OnNewData};

type MomentOf<T> = <<T as Trait>::Time as Time>::Moment;
pub type TimestampedValueOf<T> = TimestampedValue<<T as Trait>::Value, MomentOf<T>>;

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type OnNewData: OnNewData<Self::Key, Self::Value>;
	type OperatorProvider: OperatorProvider<Self::AccountId>;
	type CombineData: CombineData<Self::Key, TimestampedValueOf<Self>>;
	type Time: Time;
	type Key: Parameter + Member + Copy + Ord;
	type Value: Parameter + Member + Copy + Ord;
}

decl_storage! {
	trait Store for Module<T: Trait> as Oracle {
		pub RawValues get(raw_values): double_map T::Key, blake2_256(T::AccountId) => Option<TimestampedValueOf<T>>;
		pub HasUpdate get(has_update): map T::Key => bool;
		pub Values get(values): map T::Key => Option<TimestampedValueOf<T>>;
	}
}

decl_error! {
	// Oracle module errors
	pub enum Error {
		NoPermission,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		pub fn feed_data(origin, key: T::Key, value: T::Value) -> Result {
			let who = ensure_signed(origin)?;
			Self::_feed_data(who, key, value).map_err(|e| e.into())
		}
	}
}

decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId,
		<T as Trait>::Key,
		<T as Trait>::Value,
	{
		/// New feed data is submitted (sender, key, value)
		NewFeedData(AccountId, Key, Value),
	}
);

impl<T: Trait> Module<T> {
	pub fn read_raw_values(key: &T::Key) -> Vec<TimestampedValueOf<T>> {
		T::OperatorProvider::operators()
			.iter()
			.filter_map(|x| <RawValues<T>>::get(key, x))
			.collect()
	}

	pub fn get(key: &T::Key) -> Option<TimestampedValueOf<T>> {
		if <HasUpdate<T>>::take(key) {
			let values = Self::read_raw_values(key);
			let timestamped = T::CombineData::combine_data(key, values, <Values<T>>::get(key))?;
			<Values<T>>::insert(key, timestamped);
			return Some(timestamped);
		}
		<Values<T>>::get(key)
	}
}

impl<T: Trait> Module<T> {
	fn _feed_data(who: T::AccountId, key: T::Key, value: T::Value) -> result::Result<(), Error> {
		ensure!(T::OperatorProvider::can_feed_data(&who), Error::NoPermission);

		let timestamp = TimestampedValue {
			value,
			timestamp: T::Time::now(),
		};
		<RawValues<T>>::insert(&key, &who, timestamp);
		<HasUpdate<T>>::insert(&key, true);

		T::OnNewData::on_new_data(&key, &value);

		Self::deposit_event(RawEvent::NewFeedData(who, key, value));
		Ok(())
	}
}
