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
	traits::{CheckedAdd, Saturating, SimpleArithmetic, StaticLookup, Zero},
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
		HasNonVestingLocks,
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
		if let Some((amount, until)) = Self::locked(who) {
			T::Currency::set_lock(VESTING_LOCK_ID, who, amount, until, WithdrawReasons::all());
			amount
		} else {
			T::Currency::remove_lock(VESTING_LOCK_ID, who);
			Zero::zero()
		}
	}

	/// Returns locked balance info based on current block number: if no remaining locked balance,
	/// returns `None`, or returns `Some((amount, until))`
	fn locked(who: &T::AccountId) -> Option<(BalanceOf<T>, T::BlockNumber)> {
		let now = <frame_system::Module<T>>::block_number();
		Self::vesting_schedules(who).iter().fold(None, |acc, s| {
			let locked_amount = s.locked_amount(now);
			if locked_amount.is_zero() {
				return acc;
			}

			let s_end = s.end().expect("ensured not overflow while adding; qed");
			if let Some((amount, until)) = acc {
				Some((amount + locked_amount, until.max(s_end)))
			} else {
				Some((locked_amount, s_end))
			}
		})
	}

	fn do_add_vesting_schedule(
		from: &T::AccountId,
		to: &T::AccountId,
		schedule: VestingScheduleOf<T>,
	) -> DispatchResult {
		Self::ensure_lockable(to)?;

		let (schedule_amount, schedule_end) = Self::ensure_valid_vesting_schedule(&schedule)?;
		let (mut total_amount, mut until) = (schedule_amount, schedule_end);
		if let Some((curr_amount, curr_until)) = Self::locked(to) {
			total_amount = curr_amount
				.checked_add(&schedule_amount.into())
				.ok_or(Error::<T>::NumOverflow)?;
			until = until.max(curr_until);
		}

		T::Currency::transfer(from, to, schedule_amount, ExistenceRequirement::AllowDeath)?;
		T::Currency::set_lock(VESTING_LOCK_ID, to, total_amount, until, WithdrawReasons::all());
		<VestingSchedules<T>>::mutate(to, |v| (*v).push(schedule));

		Ok(())
	}

	fn do_update_vesting_schedules(who: &T::AccountId, schedules: Vec<VestingScheduleOf<T>>) -> DispatchResult {
		Self::ensure_lockable(who)?;

		let (total_amount, until) = schedules
			.iter()
			.try_fold::<_, _, Result<(BalanceOf<T>, T::BlockNumber), Error<T>>>(
				(Zero::zero(), Zero::zero()),
				|(acc_amount, acc_end), schedule| {
					let (amount, end) = Self::ensure_valid_vesting_schedule(schedule)?;
					Ok((acc_amount + amount, acc_end.max(end)))
				},
			)?;
		ensure!(
			T::Currency::free_balance(who) >= total_amount,
			Error::<T>::InsufficientBalanceToLock,
		);

		T::Currency::set_lock(VESTING_LOCK_ID, who, total_amount, until, WithdrawReasons::all());
		<VestingSchedules<T>>::insert(who, schedules);

		Ok(())
	}

	/// Returns `Ok((amount, end))` if valid schedule, or error.
	fn ensure_valid_vesting_schedule(
		schedule: &VestingScheduleOf<T>,
	) -> Result<(BalanceOf<T>, T::BlockNumber), Error<T>> {
		ensure!(!schedule.period.is_zero(), Error::<T>::ZeroVestingPeriod);
		ensure!(!schedule.period_count.is_zero(), Error::<T>::ZeroVestingPeriodCount);

		let amount = schedule.total_amount().ok_or(Error::<T>::NumOverflow)?;
		let schedule_end = schedule.end().ok_or(Error::<T>::NumOverflow)?;
		Ok((amount, schedule_end))
	}

	/// Ensure no other types of locks except `VESTING_LOCK_ID`.
	fn ensure_lockable(who: &T::AccountId) -> DispatchResult {
		// FIXME: use locks query in `LockableCurrency` when it's ready
		// https://github.com/paritytech/substrate/issues/4655

		// FIXME: remove `do_claim` workaround after issue #4655 fixed
		// https://github.com/paritytech/substrate/issues/4655
		let _ = Self::do_claim(who);

		let balance = T::Currency::free_balance(who);
		let locked = Self::locked(who).map_or(Zero::zero(), |(amount, _)| amount);
		let usable = balance.saturating_sub(locked);
		T::Currency::ensure_can_withdraw(who, usable, WithdrawReasons::all(), locked)
			.map_err(|_| Error::<T>::HasNonVestingLocks.into())
	}
}
