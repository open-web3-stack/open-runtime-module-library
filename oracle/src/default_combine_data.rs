use crate::{Instance, MomentOf, TimestampedValueOf, Trait};
use frame_support::traits::{Get, Time};
use orml_traits::CombineData;
use sp_std::{marker, prelude::*};

/// Sort by value and returns median timestamped value.
/// Returns prev_value if not enough valid values.
pub struct DefaultCombineData<T, I, MinimumCount, ExpiresIn>(marker::PhantomData<(T, I, MinimumCount, ExpiresIn)>);

impl<T, I, MinimumCount, ExpiresIn> CombineData<<T as Trait<I>>::OracleKey, TimestampedValueOf<T, I>>
	for DefaultCombineData<T, I, MinimumCount, ExpiresIn>
where
	T: Trait<I>,
	I: Instance,
	MinimumCount: Get<u32>,
	ExpiresIn: Get<MomentOf<T, I>>,
{
	fn combine_data(
		_key: &<T as Trait<I>>::OracleKey,
		mut values: Vec<TimestampedValueOf<T, I>>,
		prev_value: Option<TimestampedValueOf<T, I>>,
	) -> Option<TimestampedValueOf<T, I>> {
		let expires_in = ExpiresIn::get();
		let now = T::Time::now();

		values.retain(|x| x.timestamp + expires_in > now);

		let count = values.len() as u32;
		let minimum_count = MinimumCount::get();
		if count < minimum_count {
			return prev_value;
		}

		values.sort_by(|a, b| a.value.cmp(&b.value));

		let median_index = count / 2;
		Some(values[median_index as usize].clone())
	}
}
