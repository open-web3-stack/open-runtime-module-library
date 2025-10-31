pub use crate::*;

use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_std::vec;

/// Helper trait for benchmarking.
pub trait BenchmarkHelper<AccountId, Balance> {
	fn get_vesting_account_and_amount() -> Option<(AccountId, Balance)>;
}

impl<AccountId, Balance> BenchmarkHelper<AccountId, Balance> for () {
	fn get_vesting_account_and_amount() -> Option<(AccountId, Balance)> {
		None
	}
}

fn set_balance<T: Config>(who: &T::AccountId, amount: BalanceOf<T>) {
	let _ = <<T as Config>::Currency as Currency<_>>::deposit_creating(&who, amount);
}

fn total_balance<T: Config>(who: &T::AccountId) -> BalanceOf<T> {
	<<T as Config>::Currency as Currency<_>>::total_balance(who)
}

fn free_balance<T: Config>(who: &T::AccountId) -> BalanceOf<T> {
	<<T as Config>::Currency as Currency<_>>::free_balance(who)
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn vested_transfer() {
		let schedule = VestingScheduleOf::<T> {
			start: 0u32.into(),
			period: 2u32.into(),
			period_count: 3u32.into(),
			per_period: T::MinVestedTransfer::get(),
		};

		// extra 1 dollar to pay fees
		let (from, amount) = T::BenchmarkHelper::get_vesting_account_and_amount().unwrap();
		set_balance::<T>(&from, schedule.total_amount().unwrap() + amount);

		let to: T::AccountId = account("to", 0, 0);
		let to_lookup = T::Lookup::unlookup(to.clone());

		#[extrinsic_call]
		_(RawOrigin::Signed(from), to_lookup, schedule.clone());

		assert_eq!(total_balance::<T>(&to), schedule.total_amount().unwrap());
	}

	#[benchmark]
	fn claim(i: Linear<1, { T::MaxVestingSchedules::get() }>) {
		let mut schedule = VestingScheduleOf::<T> {
			start: 0u32.into(),
			period: 2u32.into(),
			period_count: 3u32.into(),
			per_period: T::MinVestedTransfer::get(),
		};

		// extra 1 dollar to pay fees
		let (from, amount) = T::BenchmarkHelper::get_vesting_account_and_amount().unwrap();
		set_balance::<T>(
			&from,
			schedule.total_amount().unwrap().saturating_mul(i.into()) + amount,
		);

		let to: T::AccountId = account("to", 0, 0);
		let to_lookup = T::Lookup::unlookup(to.clone());

		for _ in 0..i {
			schedule.start = i.into();
			assert_ok!(Pallet::<T>::vested_transfer(
				RawOrigin::Signed(from.clone()).into(),
				to_lookup.clone(),
				schedule.clone()
			));
		}
		frame_system::Pallet::<T>::set_block_number(schedule.end().unwrap() + 1u32.into());

		#[extrinsic_call]
		_(RawOrigin::Signed(to.clone()));

		assert_eq!(
			free_balance::<T>(&to),
			schedule.total_amount().unwrap().saturating_mul(i.into()),
		);
	}

	#[benchmark]
	fn update_vesting_schedules(i: Linear<1, { T::MaxVestingSchedules::get() }>) {
		let mut schedule = VestingScheduleOf::<T> {
			start: 0u32.into(),
			period: 2u32.into(),
			period_count: 3u32.into(),
			per_period: T::MinVestedTransfer::get(),
		};

		let to: T::AccountId = account("to", 0, 0);
		let to_lookup = T::Lookup::unlookup(to.clone());

		set_balance::<T>(&to, schedule.total_amount().unwrap().saturating_mul(i.into()));

		let mut schedules = vec![];
		for _ in 0..i {
			schedule.start = i.into();
			schedules.push(schedule.clone());
		}

		#[extrinsic_call]
		_(RawOrigin::Root, to_lookup, schedules);

		assert_eq!(
			free_balance::<T>(&to),
			schedule.total_amount().unwrap().saturating_mul(i.into()),
		);
	}

	impl_benchmark_test_suite! {
		Pallet,
		crate::mock::ExtBuilder::build(),
		crate::mock::Runtime,
	}
}
