use rstd::prelude::Vec;
use support::{parameter_types, traits::Time};
use traits::CombineData;

use crate::{MomentOf, TimestampedValue, Trait};

parameter_types! {
	const MinimumCount: usize = 3;
}

pub struct DefaultCombineData<T: Trait>(rstd::marker::PhantomData<T>);

impl<T: Trait> CombineData<T::Key, TimestampedValue<T::Value, MomentOf<T>>> for DefaultCombineData<T> {
	fn combine_data(
		_key: &T::Key,
		values: Vec<TimestampedValue<T::Value, MomentOf<T>>>,
		_prev_value: Option<TimestampedValue<T::Value, MomentOf<T>>>,
	) -> Option<TimestampedValue<T::Value, MomentOf<T>>> {
		let mut valid_values = values
			.into_iter()
			.filter_map(|x| {
				// TODO: check expiry
				Some(x.value)
			})
			.collect::<Vec<T::Value>>();

		valid_values.sort();

		let count = valid_values.len();
		if count < MinimumCount::get() {
			return None;
		}

		let index = count / 2;

		Some(TimestampedValue {
			value: valid_values[index],
			timestamp: T::Time::now(),
		})
	}
}
