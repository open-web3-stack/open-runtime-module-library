use rstd::prelude::Vec;
use support::{
	parameter_types,
	traits::{Get, Time},
};
use traits::CombineData;

use crate::{MomentOf, TimestampedValue, Trait};

pub struct DefaultCombineData<T: Trait>(rstd::marker::PhantomData<T>);

impl<T: Trait> CombineData<T::Key, TimestampedValue<T::Value, MomentOf<T>>> for DefaultCombineData<T> {
	fn combine_data(
		_key: &T::Key,
		values: Vec<TimestampedValue<T::Value, MomentOf<T>>>,
		prev_value: Option<TimestampedValue<T::Value, MomentOf<T>>>,
	) -> Option<TimestampedValue<T::Value, MomentOf<T>>> {
		let expires_in: MomentOf<T> = <Self as Parameters<MomentOf<T>>>::expires_in::get().into();
		let now = T::Time::now();
		let mut valid_values = values
			.into_iter()
			.filter_map(|x| {
				if x.timestamp + expires_in > now {
					return Some(x);
				}
				None
			})
			.collect::<Vec<TimestampedValue<T::Value, MomentOf<T>>>>();

		valid_values.sort_by(|a, b| a.value.cmp(&b.value));

		let count = valid_values.len();
		let minimum_count = <Self as Parameters<MomentOf<T>>>::minimum_count::get();
		if count < minimum_count {
			return prev_value;
		}

		let median_index = count / 2;
		return Some(valid_values[median_index]);
	}
}

parameter_types! {
	pub const MinimumCount: usize = 3;
	pub const ExpiresIn: u32 = 600;
}

trait Parameters<Moment> {
	type minimum_count: Get<usize>;
	type expires_in: Get<Moment>;
}

impl<T: Trait> Parameters<MomentOf<T>> for DefaultCombineData<T> {
	type minimum_count = MinimumCount;
	type expires_in = ExpiresIn;
}
