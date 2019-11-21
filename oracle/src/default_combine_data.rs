use orml_traits::CombineData;
use palette_support::{
	parameter_types,
	traits::{Get, Time},
};
use rstd::prelude::Vec;

use crate::{MomentOf, TimestampedValueOf, Trait};

/// Sort by value and returns median timestamped value.
/// Returns prev_value if not enough valid values.
pub struct DefaultCombineData<T: Trait>(rstd::marker::PhantomData<T>);

impl<T: Trait> CombineData<T::Key, TimestampedValueOf<T>> for DefaultCombineData<T> {
	fn combine_data(
		_key: &T::Key,
		values: Vec<TimestampedValueOf<T>>,
		prev_value: Option<TimestampedValueOf<T>>,
	) -> Option<TimestampedValueOf<T>> {
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
			.collect::<Vec<TimestampedValueOf<T>>>();

		let count = valid_values.len();
		let minimum_count = <Self as Parameters<MomentOf<T>>>::minimum_count::get();
		if count < minimum_count {
			return prev_value;
		}

		valid_values.sort_by(|a, b| a.value.cmp(&b.value));

		let median_index = count / 2;
		return Some(valid_values[median_index].clone());
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
