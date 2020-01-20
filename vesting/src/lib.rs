#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, HasCompact};
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::{Currency, ExistenceRequirement, LockIdentifier, LockableCurrency, WithdrawReasons},
};
use sp_std::{
	cmp::{Eq, PartialEq},
	result::Result,
	vec::Vec,
};
// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
	traits::{CheckedAdd, CheckedMul, SimpleArithmetic, StaticLookup, Zero},
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
	pub start: BlockNumber,
	pub period: BlockNumber,
	pub period_count: u32,
	#[codec(compact)]
	pub per_period: Balance,
}

impl<BlockNumber: SimpleArithmetic + Copy, Balance: SimpleArithmetic + Copy> VestingSchedule<BlockNumber, Balance> {
	/// Returns the end of all periods, `None` if calculation overflow.
	pub fn end(&self) -> Option<BlockNumber> {
		self.period
			.checked_mul(&self.period_count.into())?
			.checked_add(&self.start)
	}

	/// Returns outstanding locked balance in schedule, based on given `time`.
	///
	/// Note this func assumes schedule end calculation doesn't overflow, and it should be guaranteed by callers.
	pub fn outstanding_locked(&self, time: BlockNumber) -> Balance {
		(1..=self.period_count).fold(Zero::zero(), |acc, i| {
			let period_end = self.start + self.period * i.into();
			if period_end <= time {
				acc + self.per_period
			} else {
				acc
			}
		})
	}
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
		VestingScheduleAdded(AccountId, AccountId, VestingSchedule),
		/// Claimed vesting (who, outstanding_locked)
		Claimed(AccountId, Balance),
	}
);

decl_error! {
	/// Error for vesting module.
	pub enum Error for Module<T: Trait> {
		ZeroVestingPeriod,
		ZeroVestingPeriodCount,
		NumOverflow,
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
			Self::deposit_event(RawEvent::VestingScheduleAdded(from, to, schedule));
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
	fn do_claim(who: &T::AccountId) -> Result<BalanceOf<T>, &'static str> {
		unimplemented!()
	}

	/// Returns `(locked_balance, end)` tuple of `who`, based on current block number. `(Zero::zero(), None)`
	/// if no outstanding locked balance for vesting module.
	fn locked_balance(who: &T::AccountId) -> (BalanceOf<T>, Option<T::BlockNumber>) {
		let now = <frame_system::Module<T>>::block_number();
		Self::vesting_schedules(who)
			.iter()
			.fold((Zero::zero(), None), |(acc_locked, acc_end), s| {
				(acc_locked + s.outstanding_locked(now), acc_end.max(s.end()))
			})
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

		let (locked, locked_until) = Self::locked_balance(to);
		let new_to_lock = per_period
			.checked_mul(&period_count.into())
			.ok_or(Error::<T>::NumOverflow)?;
		let total = locked.checked_add(&new_to_lock.into()).ok_or(Error::<T>::NumOverflow)?;
		let until = {
			let schedule_end = schedule.end().ok_or(Error::<T>::NumOverflow)?;
			if let Some(l) = locked_until {
				schedule_end.max(l)
			} else {
				schedule_end
			}
		};

		T::Currency::transfer(from, to, new_to_lock, ExistenceRequirement::AllowDeath)?;
		T::Currency::set_lock(VESTING_LOCK_ID, to, total, until, WithdrawReasons::all());
		<VestingSchedules<T>>::mutate(to, |v| (*v).push(schedule));

		Ok(())
	}
}
