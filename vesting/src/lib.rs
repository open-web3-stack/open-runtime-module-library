//! # Vesting Module
//!
//! ## Overview
//!
//! Vesting module provides a means of scheduled balance lock on an account. It uses the *graded vesting* way, which
//! unlocks a specific amount of balance every period of time, until all balance unlocked.
//!
//! ### Vesting Schedule
//!
//! The schedule of a vesting is described by data structure `VestingSchedule`: from the block number of `start`, for
//! every `period` amount of blocks, `per_period` amount of balance would unlocked, until number of periods
//! `period_count` reached. Note in vesting schedules, *time* is measured by block number. All `VestingSchedule`s under
//! an account could be queried in chain state.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `add_vesting_schedule` - Add a new vesting schedule for an account.
//! - `claim` - Claim unlocked balances.
//! - `update_vesting_schedules` - Update all vesting schedules under an account, `root` origin required.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, HasCompact};
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure,
	traits::{Currency, ExistenceRequirement, LockIdentifier, LockableCurrency, WithdrawReasons},
};
use sp_std::{
	cmp::{Eq, PartialEq},
	vec::Vec,
};
// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use frame_system::{self as system, ensure_root, ensure_signed};
use sp_runtime::{
	traits::{AtLeast32Bit, CheckedAdd, StaticLookup, Zero},
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

impl<BlockNumber: AtLeast32Bit + Copy, Balance: AtLeast32Bit + Copy> VestingSchedule<BlockNumber, Balance> {
	/// Returns the end of all periods, `None` if calculation overflows.
	pub fn end(&self) -> Option<BlockNumber> {
		self.period
			.checked_mul(&self.period_count.into())?
			.checked_add(&self.start)
	}

	/// Returns all locked amount, `None` if calculation overflows.
	pub fn total_amount(&self) -> Option<Balance> {
		self.per_period.checked_mul(&self.period_count.into())
	}

	/// Returns locked amount for a given `time`.
	///
	/// Note this func assumes schedule is a valid one(non-zero period and non-overflow total amount),
	/// and it should be guaranteed by callers.
	pub fn locked_amount(&self, time: BlockNumber) -> Balance {
		let full = time
			.saturating_sub(self.start)
			.checked_div(&self.period)
			.expect("ensured non-zero period; qed");
		let unrealized = self.period_count.saturating_sub(full.unique_saturated_into());
		self.per_period
			.checked_mul(&unrealized.into())
			.expect("ensured non-overflow total amount; qed")
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
		}): map hasher(blake2_128_concat) T::AccountId => Vec<VestingScheduleOf<T>>;
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
		/// Claimed vesting (who, locked_amount)
		Claimed(AccountId, Balance),
		/// Updated vesting schedules (who)
		VestingSchedulesUpdated(AccountId),
	}
);

decl_error! {
	/// Error for vesting module.
	pub enum Error for Module<T: Trait> {
		ZeroVestingPeriod,
		ZeroVestingPeriodCount,
		NumOverflow,
		InsufficientBalanceToLock,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		pub fn claim(origin) {
			let who = ensure_signed(origin)?;
			let locked_amount = Self::do_claim(&who);

			Self::deposit_event(RawEvent::Claimed(who, locked_amount));
		}

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
		) {
			ensure_root(origin)?;

			let account = T::Lookup::lookup(who)?;
			Self::do_update_vesting_schedules(&account, vesting_schedules)?;

			Self::deposit_event(RawEvent::VestingSchedulesUpdated(account));
		}
	}
}

const VESTING_LOCK_ID: LockIdentifier = *b"ormlvest";

impl<T: Trait> Module<T> {
	fn do_claim(who: &T::AccountId) -> BalanceOf<T> {
		let locked = Self::locked_balance(who);
		if locked.is_zero() {
			T::Currency::remove_lock(VESTING_LOCK_ID, who);
		} else {
			T::Currency::set_lock(VESTING_LOCK_ID, who, locked, WithdrawReasons::all());
		}
		locked
	}

	/// Returns locked balance based on current block number.
	fn locked_balance(who: &T::AccountId) -> BalanceOf<T> {
		let now = <frame_system::Module<T>>::block_number();
		Self::vesting_schedules(who)
			.iter()
			.fold(Zero::zero(), |acc, s| acc + s.locked_amount(now))
	}

	fn do_add_vesting_schedule(
		from: &T::AccountId,
		to: &T::AccountId,
		schedule: VestingScheduleOf<T>,
	) -> DispatchResult {
		let schedule_amount = Self::ensure_valid_vesting_schedule(&schedule)?;
		let total_amount = Self::locked_balance(to)
			.checked_add(&schedule_amount.into())
			.ok_or(Error::<T>::NumOverflow)?;

		T::Currency::transfer(from, to, schedule_amount, ExistenceRequirement::AllowDeath)?;
		T::Currency::set_lock(VESTING_LOCK_ID, to, total_amount, WithdrawReasons::all());
		<VestingSchedules<T>>::mutate(to, |v| (*v).push(schedule));

		Ok(())
	}

	fn do_update_vesting_schedules(who: &T::AccountId, schedules: Vec<VestingScheduleOf<T>>) -> DispatchResult {
		let total_amount = schedules.iter().try_fold::<_, _, Result<BalanceOf<T>, Error<T>>>(
			Zero::zero(),
			|acc_amount, schedule| {
				let amount = Self::ensure_valid_vesting_schedule(schedule)?;
				Ok(acc_amount + amount)
			},
		)?;
		ensure!(
			T::Currency::free_balance(who) >= total_amount,
			Error::<T>::InsufficientBalanceToLock,
		);

		T::Currency::set_lock(VESTING_LOCK_ID, who, total_amount, WithdrawReasons::all());
		<VestingSchedules<T>>::insert(who, schedules);

		Ok(())
	}

	/// Returns `Ok(amount)` if valid schedule, or error.
	fn ensure_valid_vesting_schedule(schedule: &VestingScheduleOf<T>) -> Result<BalanceOf<T>, Error<T>> {
		ensure!(!schedule.period.is_zero(), Error::<T>::ZeroVestingPeriod);
		ensure!(!schedule.period_count.is_zero(), Error::<T>::ZeroVestingPeriodCount);
		ensure!(schedule.end().is_some(), Error::<T>::NumOverflow);

		schedule.total_amount().ok_or(Error::<T>::NumOverflow)
	}
}
