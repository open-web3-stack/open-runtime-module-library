//! # Vesting Module
//!
//! ## Overview
//!
//! Vesting module provides a means of scheduled balance lock on an account. It
//! uses the *graded vesting* way, which unlocks a specific amount of balance
//! every period of time, until all balance unlocked.
//!
//! ### Vesting Schedule
//!
//! The schedule of a vesting is described by data structure `VestingSchedule`:
//! from the block number of `start`, for every `period` amount of blocks,
//! `per_period` amount of balance would unlocked, until number of periods
//! `period_count` reached. Note in vesting schedules, *time* is measured by
//! block number. All `VestingSchedule`s under an account could be queried in
//! chain state.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! - `vested_transfer` - Add a new vesting schedule for an account.
//! - `claim` - Claim unlocked balances.
//! - `update_vesting_schedules` - Update all vesting schedules under an
//!   account, `root` origin required.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use codec::{HasCompact, MaxEncodedLen};
use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{Currency, EnsureOrigin, ExistenceRequirement, Get, LockIdentifier, LockableCurrency, WithdrawReasons},
	transactional, BoundedVec,
};
use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AtLeast32Bit, BlockNumberProvider, CheckedAdd, Saturating, StaticLookup, Zero},
	ArithmeticError, DispatchResult, RuntimeDebug,
};
use sp_std::{
	cmp::{Eq, PartialEq},
	vec::Vec,
};

mod mock;
mod tests;
mod weights;

pub use module::*;
pub use weights::WeightInfo;

pub const VESTING_LOCK_ID: LockIdentifier = *b"ormlvest";

/// The vesting schedule.
///
/// Benefits would be granted gradually, `per_period` amount every `period`
/// of blocks after `start`.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct VestingSchedule<BlockNumber, Balance: HasCompact> {
	/// Vesting starting block
	pub start: BlockNumber,
	/// Number of blocks between vest
	pub period: BlockNumber,
	/// Number of vest
	pub period_count: u32,
	/// Amount of tokens to release per vest
	#[codec(compact)]
	pub per_period: Balance,
}

impl<BlockNumber: AtLeast32Bit + Copy, Balance: AtLeast32Bit + Copy> VestingSchedule<BlockNumber, Balance> {
	/// Returns the end of all periods, `None` if calculation overflows.
	pub fn end(&self) -> Option<BlockNumber> {
		// period * period_count + start
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
	/// Note this func assumes schedule is a valid one(non-zero period and
	/// non-overflow total amount), and it should be guaranteed by callers.
	pub fn locked_amount(&self, time: BlockNumber) -> Balance {
		// full = (time - start) / period
		// unrealized = period_count - full
		// per_period * unrealized
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

#[frame_support::pallet]
pub mod module {
	use super::*;

	pub(crate) type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	pub(crate) type VestingScheduleOf<T> = VestingSchedule<<T as frame_system::Config>::BlockNumber, BalanceOf<T>>;
	pub type ScheduledItem<T> = (
		<T as frame_system::Config>::AccountId,
		<T as frame_system::Config>::BlockNumber,
		<T as frame_system::Config>::BlockNumber,
		u32,
		BalanceOf<T>,
	);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

		#[pallet::constant]
		/// The minimum amount transferred to call `vested_transfer`.
		type MinVestedTransfer: Get<BalanceOf<Self>>;

		/// Required origin for vested transfer.
		type VestedTransferOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;

		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;

		/// The maximum vesting schedules
		type MaxVestingSchedules: Get<u32>;

		// The block number provider
		type BlockNumberProvider: BlockNumberProvider<BlockNumber = Self::BlockNumber>;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Vesting period is zero
		ZeroVestingPeriod,
		/// Number of vests is zero
		ZeroVestingPeriodCount,
		/// Insufficient amount of balance to lock
		InsufficientBalanceToLock,
		/// This account have too many vesting schedules
		TooManyVestingSchedules,
		/// The vested transfer amount is too low
		AmountLow,
		/// Failed because the maximum vesting schedules was exceeded
		MaxVestingSchedulesExceeded,
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// Added new vesting schedule.
		VestingScheduleAdded {
			from: T::AccountId,
			to: T::AccountId,
			vesting_schedule: VestingScheduleOf<T>,
		},
		/// Claimed vesting.
		Claimed { who: T::AccountId, amount: BalanceOf<T> },
		/// Updated vesting schedules.
		VestingSchedulesUpdated { who: T::AccountId },
	}

	/// Vesting schedules of an account.
	///
	/// VestingSchedules: map AccountId => Vec<VestingSchedule>
	#[pallet::storage]
	#[pallet::getter(fn vesting_schedules)]
	pub type VestingSchedules<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<VestingScheduleOf<T>, T::MaxVestingSchedules>,
		ValueQuery,
	>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub vesting: Vec<ScheduledItem<T>>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { vesting: vec![] }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			self.vesting
				.iter()
				.for_each(|(who, start, period, period_count, per_period)| {
					let mut bounded_schedules = VestingSchedules::<T>::get(who);
					bounded_schedules
						.try_push(VestingSchedule {
							start: *start,
							period: *period,
							period_count: *period_count,
							per_period: *per_period,
						})
						.expect("Max vesting schedules exceeded");
					let total_amount = bounded_schedules
						.iter()
						.try_fold::<_, _, Result<BalanceOf<T>, DispatchError>>(Zero::zero(), |acc_amount, schedule| {
							let amount = ensure_valid_vesting_schedule::<T>(schedule)?;
							Ok(acc_amount + amount)
						})
						.expect("Invalid vesting schedule");

					assert!(
						T::Currency::free_balance(who) >= total_amount,
						"Account do not have enough balance"
					);

					T::Currency::set_lock(VESTING_LOCK_ID, who, total_amount, WithdrawReasons::all());
					VestingSchedules::<T>::insert(who, bounded_schedules);
				});
		}
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::claim((<T as Config>::MaxVestingSchedules::get() / 2) as u32))]
		pub fn claim(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let locked_amount = Self::do_claim(&who);

			Self::deposit_event(Event::Claimed {
				who,
				amount: locked_amount,
			});
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::vested_transfer())]
		pub fn vested_transfer(
			origin: OriginFor<T>,
			dest: <T::Lookup as StaticLookup>::Source,
			schedule: VestingScheduleOf<T>,
		) -> DispatchResult {
			let from = T::VestedTransferOrigin::ensure_origin(origin)?;
			let to = T::Lookup::lookup(dest)?;
			Self::do_vested_transfer(&from, &to, schedule.clone())?;

			Self::deposit_event(Event::VestingScheduleAdded {
				from,
				to,
				vesting_schedule: schedule,
			});
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::update_vesting_schedules(vesting_schedules.len() as u32))]
		pub fn update_vesting_schedules(
			origin: OriginFor<T>,
			who: <T::Lookup as StaticLookup>::Source,
			vesting_schedules: Vec<VestingScheduleOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			let account = T::Lookup::lookup(who)?;
			Self::do_update_vesting_schedules(&account, vesting_schedules)?;

			Self::deposit_event(Event::VestingSchedulesUpdated { who: account });
			Ok(())
		}

		#[pallet::weight(T::WeightInfo::claim((<T as Config>::MaxVestingSchedules::get() / 2) as u32))]
		pub fn claim_for(origin: OriginFor<T>, dest: <T::Lookup as StaticLookup>::Source) -> DispatchResult {
			let _ = ensure_signed(origin)?;
			let who = T::Lookup::lookup(dest)?;
			let locked_amount = Self::do_claim(&who);

			Self::deposit_event(Event::Claimed {
				who,
				amount: locked_amount,
			});
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn do_claim(who: &T::AccountId) -> BalanceOf<T> {
		let locked = Self::locked_balance(who);
		if locked.is_zero() {
			// cleanup the storage and unlock the fund
			<VestingSchedules<T>>::remove(who);
			T::Currency::remove_lock(VESTING_LOCK_ID, who);
		} else {
			T::Currency::set_lock(VESTING_LOCK_ID, who, locked, WithdrawReasons::all());
		}
		locked
	}

	/// Returns locked balance based on current block number.
	fn locked_balance(who: &T::AccountId) -> BalanceOf<T> {
		let now = T::BlockNumberProvider::current_block_number();
		<VestingSchedules<T>>::mutate_exists(who, |maybe_schedules| {
			let total = if let Some(schedules) = maybe_schedules.as_mut() {
				let mut total: BalanceOf<T> = Zero::zero();
				schedules.retain(|s| {
					let amount = s.locked_amount(now);
					total = total.saturating_add(amount);
					!amount.is_zero()
				});
				total
			} else {
				Zero::zero()
			};
			if total.is_zero() {
				*maybe_schedules = None;
			}
			total
		})
	}

	#[transactional]
	fn do_vested_transfer(from: &T::AccountId, to: &T::AccountId, schedule: VestingScheduleOf<T>) -> DispatchResult {
		let schedule_amount = ensure_valid_vesting_schedule::<T>(&schedule)?;

		let total_amount = Self::locked_balance(to)
			.checked_add(&schedule_amount)
			.ok_or(ArithmeticError::Overflow)?;

		T::Currency::transfer(from, to, schedule_amount, ExistenceRequirement::AllowDeath)?;
		T::Currency::set_lock(VESTING_LOCK_ID, to, total_amount, WithdrawReasons::all());
		<VestingSchedules<T>>::try_append(to, schedule).map_err(|_| Error::<T>::MaxVestingSchedulesExceeded)?;
		Ok(())
	}

	fn do_update_vesting_schedules(who: &T::AccountId, schedules: Vec<VestingScheduleOf<T>>) -> DispatchResult {
		let bounded_schedules: BoundedVec<VestingScheduleOf<T>, T::MaxVestingSchedules> = schedules
			.try_into()
			.map_err(|_| Error::<T>::MaxVestingSchedulesExceeded)?;

		// empty vesting schedules cleanup the storage and unlock the fund
		if bounded_schedules.len().is_zero() {
			<VestingSchedules<T>>::remove(who);
			T::Currency::remove_lock(VESTING_LOCK_ID, who);
			return Ok(());
		}

		let total_amount = bounded_schedules
			.iter()
			.try_fold::<_, _, Result<BalanceOf<T>, DispatchError>>(Zero::zero(), |acc_amount, schedule| {
				let amount = ensure_valid_vesting_schedule::<T>(schedule)?;
				Ok(acc_amount + amount)
			})?;
		ensure!(
			T::Currency::free_balance(who) >= total_amount,
			Error::<T>::InsufficientBalanceToLock,
		);

		T::Currency::set_lock(VESTING_LOCK_ID, who, total_amount, WithdrawReasons::all());
		<VestingSchedules<T>>::insert(who, bounded_schedules);

		Ok(())
	}
}

/// Returns `Ok(total_total)` if valid schedule, or error.
fn ensure_valid_vesting_schedule<T: Config>(schedule: &VestingScheduleOf<T>) -> Result<BalanceOf<T>, DispatchError> {
	ensure!(!schedule.period.is_zero(), Error::<T>::ZeroVestingPeriod);
	ensure!(!schedule.period_count.is_zero(), Error::<T>::ZeroVestingPeriodCount);
	ensure!(schedule.end().is_some(), ArithmeticError::Overflow);

	let total_total = schedule.total_amount().ok_or(ArithmeticError::Overflow)?;

	ensure!(total_total >= T::MinVestedTransfer::get(), Error::<T>::AmountLow);

	Ok(total_total)
}
