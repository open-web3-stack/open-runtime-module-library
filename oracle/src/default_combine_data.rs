use frame_support::traits::{Get, Time};
use orml_traits::CombineData;
use sp_std::{marker, prelude::*};

use crate::{MomentOf, TimestampedValueOf, Trait};

/// Sort by value and returns median timestamped value.
/// Returns prev_value if not enough valid values.
pub struct DefaultCombineData<T, MinimumCount, ExpiresIn>(marker::PhantomData<(T, MinimumCount, ExpiresIn)>);

impl<T, MinimumCount, ExpiresIn> CombineData<T::OracleKey, TimestampedValueOf<T>>
	for DefaultCombineData<T, MinimumCount, ExpiresIn>
where
	T: Trait,
	MinimumCount: Get<u32>,
	ExpiresIn: Get<MomentOf<T>>,
{
	fn combine_data(
		_key: &T::OracleKey,
		values: Vec<TimestampedValueOf<T>>,
		prev_value: Option<TimestampedValueOf<T>>,
	) -> Option<TimestampedValueOf<T>> {
		let expires_in = ExpiresIn::get();
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

		let count = valid_values.len() as u32;
		let minimum_count = MinimumCount::get();
		if count < minimum_count {
			return prev_value;
		}

		valid_values.sort_by(|a, b| a.value.cmp(&b.value));

		let median_index = count / 2;
		return Some(valid_values[median_index as usize].clone());
	}
}
