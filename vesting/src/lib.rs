#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage,
	traits::{Currency, LockableCurrency},
};
use rstd::{
	cmp::{Eq, PartialEq},
	vec::Vec,
};
// FIXME: `pallet/frame-` prefix should be used for all pallet modules, but currently `frame_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3295 https://github.com/paritytech/substrate/issues/3295
use frame_system::{self as system, ensure_signed};
use sp_runtime::{traits::StaticLookup, DispatchResult, RuntimeDebug};

#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug)]
pub struct VestingSchedule<BlockNumber, Balance> {
	start: BlockNumber,
	period: BlockNumber,
	amount: Balance,
}

pub type BalanceOf<T> = <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
pub type VestingScheduleOf<T> = VestingSchedule<<T as frame_system::Trait>::BlockNumber, BalanceOf<T>>;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type Currency: LockableCurrency<Self::AccountId>;
}

decl_storage! {
	trait Store for Module<T: Trait> as Vesting {
		/// Vesting schedules of an account.
		pub VestingSchedules get(fn vesting_schedules): map T::AccountId => Vec<VestingScheduleOf<T>>;
	}
}

decl_event!(
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
		<T as frame_system::Trait>::BlockNumber,
		Balance = BalanceOf<T>
	{
		/// Added new vesting schedule (who, start, period, amount)
		VestingScheduleAdded(AccountId, BlockNumber, BlockNumber, Balance),
	}
);

decl_error! {
	/// Error for vesting module.
	pub enum Error for Module<T: Trait> {
		InVestingPeriod,
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
			start: T::BlockNumber,
			period: T::BlockNumber,
			amount: BalanceOf<T>,
		) {}

		pub fn update_vesting_schedules(origin, who: <T::Lookup as StaticLookup>::Source, vesting_schedules: Vec<VestingScheduleOf<T>>) {}
	}
}

impl<T: Trait> Module<T> {
	fn do_claim(who: &T::AccountId) -> DispatchResult {
		Ok(())
	}
}
