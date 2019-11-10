use crate::{MomentOf, TimestampedValue, Trait};
use support::{parameter_types, traits::Time};
use traits::CombineData;

parameter_types! {
	pub const MinimumCount: usize = 3;
}

pub struct DefaultCombineData<T: Trait>(rstd::marker::PhantomData<T>);

impl<T: Trait> CombineData<T::Key, TimestampedValue<T::Value, MomentOf<T>>> for DefaultCombineData<T> {
	fn combine_data(
		key: &T::Key,
		values: Vec<TimestampedValue<T::Value, MomentOf<T>>>,
		prev_value: Option<TimestampedValue<T::Value, MomentOf<T>>>,
	) -> Option<TimestampedValue<T::Value, MomentOf<T>>> {
		// TODO: filter valid values and sort by value
		let count = values.len();
		if count < MinimumCount::get() {
			return None;
		}
		let index = count / 2;
		Some(TimestampedValue {
			value: values[index].value,
			timestamp: T::Time::now(),
		})
	}
}
