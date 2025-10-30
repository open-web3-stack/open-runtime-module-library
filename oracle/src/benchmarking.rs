pub use crate::*;

use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::{Pallet as System, RawOrigin};

/// Helper trait for benchmarking.
pub trait BenchmarkHelper<OracleKey, OracleValue, L: Get<u32>> {
	/// Returns a list of `(oracle_key, oracle_value)` pairs to be used for
	/// benchmarking.
	///
	/// NOTE: User should ensure to at least submit two values, otherwise the
	/// benchmark linear analysis might fail.
	fn get_currency_id_value_pairs() -> BoundedVec<(OracleKey, OracleValue), L>;
}

impl<OracleKey, OracleValue, L: Get<u32>> BenchmarkHelper<OracleKey, OracleValue, L> for () {
	fn get_currency_id_value_pairs() -> BoundedVec<(OracleKey, OracleValue), L> {
		BoundedVec::default()
	}
}

#[instance_benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn feed_values(x: Linear<0, { T::BenchmarkHelper::get_currency_id_value_pairs().len() as u32 }>) {
		// Register the caller
		let caller: T::AccountId = whitelisted_caller();
		T::Members::add(&caller);

		let values = T::BenchmarkHelper::get_currency_id_value_pairs()[..x as usize]
			.to_vec()
			.try_into()
			.expect("Must succeed since at worst the length remained the same.");

		#[extrinsic_call]
		_(RawOrigin::Signed(caller.clone()), values);

		assert!(HasDispatched::<T, I>::get().contains(&caller));
	}

	#[benchmark]
	fn on_finalize() {
		// Register the caller
		let caller: T::AccountId = whitelisted_caller();
		T::Members::add(&caller);

		// Feed some values before running `on_finalize` hook
		System::<T>::set_block_number(1u32.into());
		let values = T::BenchmarkHelper::get_currency_id_value_pairs();
		assert_ok!(Pallet::<T, I>::feed_values(RawOrigin::Signed(caller).into(), values));

		#[block]
		{
			Pallet::<T, I>::on_finalize(System::<T>::block_number());
		}

		assert!(!HasDispatched::<T, I>::exists());
	}

	impl_benchmark_test_suite! {
		Pallet,
		crate::mock::new_test_ext(),
		crate::mock::Test,
	}
}
