#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod on_new_data;
mod operator_provider;
mod tests;
mod timestamped_value;

pub use on_new_data::OnNewData;
pub use operator_provider::OperatorProvider;
use rstd::prelude::Vec;
use sr_primitives::traits::Member;
use support::{decl_event, decl_module, decl_storage, dispatch::Result, ensure, traits::Time, Parameter};
use system::ensure_signed;
pub use timestamped_value::TimestampedValue;

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type OperatorProvider: OperatorProvider<Self::AccountId>;
	type Key: Parameter + Member + Copy;
	type Value: Parameter + Member + Copy;
	type Time: Time;
	type OnNewData: OnNewData<Self::Key, Self::Value>;
}

type MomentOf<T> = <<T as Trait>::Time as Time>::Moment;

decl_storage! {
	trait Store for Module<T: Trait> as Oracle {
		pub RawValues get(raw_values): map (T::AccountId, T::Key) => Option<TimestampedValue<T::Value, MomentOf<T>>>;
		pub HasUpdate get(has_update): map T::Key => bool;
		pub Values get(values): map T::Key => Option<T::Value>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		pub fn feed_data(origin, key: T::Key, value: T::Value) -> Result {
			let who = ensure_signed(origin)?;
			ensure!(T::OperatorProvider::can_feed_data(&who), "No permission to feed data");
			Self::_feed_data(who, key, value)
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
}

impl<T: Trait> Module<T> {
	fn _feed_data(who: T::AccountId, key: T::Key, value: T::Value) -> Result {
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
