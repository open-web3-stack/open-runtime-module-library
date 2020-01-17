#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, HasCompact};
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::{Currency, ExistenceRequirement, LockIdentifier, LockableCurrency, WithdrawReasons},
};
use rstd::{
	cmp::{Eq, PartialEq},
	vec::Vec,
};
// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
	traits::{StaticLookup, Zero},
	DispatchResult, RuntimeDebug,
};

mod mock;
mod tests;

/// The vesting schedule.
///
/// Benefits would be granted gradually, `per_period` amount every `period` of blocks
/// after `start`.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug)]
pub struct VestingSchedule<BlockNumber, Balance: HasCompact> {
	start: BlockNumber,
	period: BlockNumber,
	period_count: u32,
	#[codec(compact)]
	per_period: Balance,
}

pub type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
pub type VestingScheduleOf<T> = VestingSchedule<<T as frame_system::Trait>::BlockNumber, BalanceOf<T>>;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Vesting {
		/// Vesting schedules of an account.
		pub VestingSchedules get(fn vesting_schedules) build(|config: &GenesisConfig<T>| {
			config.vesting.iter()
				.map(|&(ref who, start, period, period_count, per_period)|
					(who.clone(), vec![VestingSchedule {start, period, period_count, per_period}])
				)
				.collect::<Vec<_>>()
		}): map T::AccountId => Vec<VestingScheduleOf<T>>;
	}

	add_extra_genesis {
		config(vesting): Vec<(T::AccountId, T::BlockNumber, T::BlockNumber, u32, BalanceOf<T>)>;
	}
}

decl_event!(
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
		Balance = BalanceOf<T>,
		VestingSchedule = VestingScheduleOf<T>
	{
		/// Added new vesting schedule (from, to, vesting_schedule)
		Vested(AccountId, AccountId, VestingSchedule),
		/// Claimed vesting (who, outstanding_locked)
		Claimed(AccountId, Balance),
	}
);

decl_error! {
	/// Error for vesting module.
	pub enum Error for Module<T: Trait> {
		ZeroVestingPeriod,
		ZeroVestingPeriodCount,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		pub fn claim(origin) {}

		pub fn add_vesting_schedule(
			origin,
			dest: <T::Lookup as StaticLookup>::Source,
			schedule: VestingScheduleOf<T>,
		) {
			let from = ensure_signed(origin)?;
			let to = T::Lookup::lookup(dest)?;
			Self::do_add_vesting_schedule(&from, &to, schedule.clone())?;
			Self::deposit_event(RawEvent::Vested(from, to, schedule));
		}

		pub fn update_vesting_schedules(
			origin,
			who: <T::Lookup as StaticLookup>::Source,
			vesting_schedules: Vec<VestingScheduleOf<T>>
		) {}
	}
}

const VESTING_LOCK_ID: LockIdentifier = *b"vestingm";

impl<T: Trait> Module<T> {
	fn do_claim(who: &T::AccountId) -> DispatchResult {
		//		let mut amount = Zero::zero();
		//		let outstanding_schedules = Self::vesting_schedules().into_iter().filter_map(|v| {
		//			None
		//		});

		Ok(())
	}

	fn do_add_vesting_schedule(
		from: &T::AccountId,
		to: &T::AccountId,
		schedule: VestingScheduleOf<T>,
	) -> DispatchResult {
		let VestingSchedule {
			start,
			period,
			period_count,
			per_period,
		} = schedule;

		ensure!(!period.is_zero(), Error::<T>::ZeroVestingPeriod);
		ensure!(!period_count.is_zero(), Error::<T>::ZeroVestingPeriodCount);

		//TODO: ensure no existing locks, or only `VESTING_LOCK_ID` locks.

		let amount = per_period * period_count.into();
		T::Currency::transfer(from, to, amount, ExistenceRequirement::AllowDeath)?;

		let until = start + period * period_count.into();
		T::Currency::set_lock(VESTING_LOCK_ID, to, amount, until, WithdrawReasons::all());

		<VestingSchedules<T>>::mutate(to, |v| (*v).push(schedule));

		Ok(())
	}
}
