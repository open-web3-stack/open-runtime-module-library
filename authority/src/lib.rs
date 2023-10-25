//! # Authority
//! A module to provide features for governance including dispatch method on
//! behalf of other accounts and schedule dispatchables.
//!
//! - [`Config`](./trait.Config.html)
//! - [`Call`](./enum.Call.html)
//! - [`Module`](./struct.Module.html)
//!
//! ## Overview
//!
//! Two functionalities are provided by this module:
//! - schedule a dispatchable
//! - dispatch method with on behalf of other origins
//!
//! NOTE:
//!
//! In order to derive a feasible max encoded len for `DelayedOrigin`, it is
//! assumed that there are no nested `DelayedOrigin` in `OriginCaller`.
//! In practice, this means there should not be nested `schedule_dispatch`.
//! Otherwise the proof size estimation may not be accurate.

#![cfg_attr(not(feature = "std"), no_std)]
// Disable the following three lints since they originate from an external macro
#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::boxed_local)]
#![allow(clippy::borrowed_box)]
#![allow(clippy::unused_unit)]

use frame_support::{
	dispatch::PostDispatchInfo,
	dispatch::{DispatchClass, GetDispatchInfo, Pays},
	pallet_prelude::*,
	traits::{
		schedule::{v1::Named as ScheduleNamed, DispatchTime, Priority},
		EitherOfDiverse, EnsureOrigin, Get, IsType, OriginTrait,
	},
};
use frame_system::{pallet_prelude::*, EnsureRoot, EnsureSigned};
use parity_scale_codec::MaxEncodedLen;
use scale_info::TypeInfo;
use sp_core::defer;
use sp_runtime::{
	traits::{CheckedSub, Dispatchable, Hash, Saturating},
	ArithmeticError, DispatchError, DispatchResult, Either, RuntimeDebug,
};
use sp_std::prelude::*;

mod mock;
mod tests;
mod weights;

pub use weights::WeightInfo;

/// A delayed origin. Can only be dispatched via `dispatch_as` with a delay.
#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode, TypeInfo)]
pub struct DelayedOrigin<BlockNumber, PalletsOrigin> {
	/// Number of blocks that this call have been delayed.
	pub(crate) delay: BlockNumber,
	/// The initial origin.
	pub(crate) origin: Box<PalletsOrigin>,
}

#[cfg(any(feature = "std", feature = "runtime-benchmarks", test))]
impl<BlockNumber, PalletsOrigin> DelayedOrigin<BlockNumber, PalletsOrigin> {
	pub fn new(delay: BlockNumber, origin: Box<PalletsOrigin>) -> Self {
		Self { delay, origin }
	}
}

#[cfg(feature = "std")]
mod helper {
	use std::cell::RefCell;

	thread_local! {
		static NESTED_MAX_ENCODED_LEN: RefCell<bool> = RefCell::new(false);
	}

	pub fn set_nested_max_encoded_len(val: bool) {
		NESTED_MAX_ENCODED_LEN.with(|v| *v.borrow_mut() = val);
	}

	pub fn nested_max_encoded_len() -> bool {
		NESTED_MAX_ENCODED_LEN.with(|v| *v.borrow())
	}
}

#[cfg(not(feature = "std"))]
mod helper {
	static mut NESTED_MAX_ENCODED_LEN: bool = false;

	pub fn set_nested_max_encoded_len(val: bool) {
		unsafe {
			NESTED_MAX_ENCODED_LEN = val;
		}
	}

	pub fn nested_max_encoded_len() -> bool {
		unsafe { NESTED_MAX_ENCODED_LEN }
	}
}

// Manual implementation to break recursive calls of `MaxEncodedLen` as the
// implementation of `PalletsOrigin::max_encoded_len` will also call
// `MaxEncodedLen` on `DelayedOrigin`. This is only safe if there are no nested
// `DelayedOrigin`. It is only possible to construct a `DelayedOrigin` via
// `schedule_dispatch` which is a protected call only accessible via governance.
impl<BlockNumber: MaxEncodedLen, PalletsOrigin: MaxEncodedLen> MaxEncodedLen
	for DelayedOrigin<BlockNumber, PalletsOrigin>
{
	fn max_encoded_len() -> usize {
		if helper::nested_max_encoded_len() {
			return 0;
		}

		helper::set_nested_max_encoded_len(true);
		defer!(helper::set_nested_max_encoded_len(false));

		BlockNumber::max_encoded_len() + PalletsOrigin::max_encoded_len()
	}
}

/// Ensure the origin have a minimum amount of delay.
pub struct EnsureDelayed<Delay, Inner, BlockNumber, PalletsOrigin>(
	sp_std::marker::PhantomData<(Delay, Inner, BlockNumber, PalletsOrigin)>,
);
impl<
		PalletsOrigin: Into<O>,
		O: Into<Result<DelayedOrigin<BlockNumber, PalletsOrigin>, O>> + From<DelayedOrigin<BlockNumber, PalletsOrigin>>,
		Delay: Get<BlockNumber>,
		Inner: EnsureOrigin<O>,
		BlockNumber: PartialOrd,
	> EnsureOrigin<O> for EnsureDelayed<Delay, Inner, BlockNumber, PalletsOrigin>
{
	type Success = Inner::Success;

	fn try_origin(o: O) -> Result<Self::Success, O> {
		o.into().and_then(|delayed_origin| {
			if delayed_origin.delay >= Delay::get() {
				let pallets_origin = *delayed_origin.origin;
				Inner::try_origin(pallets_origin.into())
			} else {
				Err(delayed_origin.into())
			}
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<O, ()> {
		unimplemented!()
	}
}

/// Config for orml-authority
pub trait AuthorityConfig<Origin, PalletsOrigin, BlockNumber> {
	/// Check if the `origin` is allowed to schedule a dispatchable call
	/// with a given `priority`.
	fn check_schedule_dispatch(origin: Origin, priority: Priority) -> DispatchResult;
	/// Check if the `origin` is allow to fast track a scheduled task that
	/// initially created by `initial_origin`. `new_delay` is number of
	/// blocks this dispatchable will be dispatched from now after fast
	/// track.
	fn check_fast_track_schedule(
		origin: Origin,
		initial_origin: &PalletsOrigin,
		new_delay: BlockNumber,
	) -> DispatchResult;
	/// Check if the `origin` is allow to delay a scheduled task that
	/// initially created by `initial_origin`.
	fn check_delay_schedule(origin: Origin, initial_origin: &PalletsOrigin) -> DispatchResult;
	/// Check if the `origin` is allow to cancel a scheduled task that
	/// initially created by `initial_origin`.
	fn check_cancel_schedule(origin: Origin, initial_origin: &PalletsOrigin) -> DispatchResult;
}

/// Represent an origin that can be dispatched by other origins with
/// permission check.
pub trait AsOriginId<Origin, PalletsOrigin> {
	/// Convert into `PalletsOrigin`
	fn into_origin(self) -> PalletsOrigin;
	/// Check if the `origin` is allow to dispatch call on behalf of this
	/// origin.
	fn check_dispatch_from(&self, origin: Origin) -> DispatchResult;
}

/// The schedule task index type.
pub type ScheduleTaskIndex = u32;

pub use module::*;

#[frame_support::pallet]
pub mod module {
	use super::*;

	/// Origin for the authority module.
	#[pallet::origin]
	pub type Origin<T> = DelayedOrigin<BlockNumberFor<T>, <T as Config>::PalletsOrigin>;
	pub(crate) type CallOf<T> = <T as Config>::RuntimeCall;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The outer origin type.
		type RuntimeOrigin: From<DelayedOrigin<BlockNumberFor<Self>, <Self as Config>::PalletsOrigin>>
			+ IsType<<Self as frame_system::Config>::RuntimeOrigin>
			+ OriginTrait<PalletsOrigin = Self::PalletsOrigin>;

		/// The caller origin, overarching type of all pallets origins.
		type PalletsOrigin: Parameter + Into<<Self as frame_system::Config>::RuntimeOrigin>;

		/// The aggregated call type.
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = <Self as frame_system::Config>::RuntimeOrigin, PostInfo = PostDispatchInfo>
			+ GetDispatchInfo;

		/// The Scheduler.
		type Scheduler: ScheduleNamed<BlockNumberFor<Self>, <Self as Config>::RuntimeCall, Self::PalletsOrigin>;

		/// The type represent origin that can be dispatched by other origins.
		type AsOriginId: Parameter + AsOriginId<<Self as frame_system::Config>::RuntimeOrigin, Self::PalletsOrigin>;

		/// Additional permission config.
		type AuthorityConfig: AuthorityConfig<
			<Self as frame_system::Config>::RuntimeOrigin,
			Self::PalletsOrigin,
			BlockNumberFor<Self>,
		>;

		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Failed to schedule a task.
		FailedToSchedule,
		/// Failed to cancel a task.
		FailedToCancel,
		/// Failed to fast track a task.
		FailedToFastTrack,
		/// Failed to delay a task.
		FailedToDelay,
		/// Call is not authorized.
		CallNotAuthorized,
		/// Triggering the call is not permitted.
		TriggerCallNotPermitted,
		/// Call weight bound is wrong.
		WrongCallWeightBound,
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// A call is dispatched.
		Dispatched { result: DispatchResult },
		/// A call is scheduled.
		Scheduled {
			origin: T::PalletsOrigin,
			index: ScheduleTaskIndex,
		},
		/// A scheduled call is fast tracked.
		FastTracked {
			origin: T::PalletsOrigin,
			index: ScheduleTaskIndex,
			when: BlockNumberFor<T>,
		},
		/// A scheduled call is delayed.
		Delayed {
			origin: T::PalletsOrigin,
			index: ScheduleTaskIndex,
			when: BlockNumberFor<T>,
		},
		/// A scheduled call is cancelled.
		Cancelled {
			origin: T::PalletsOrigin,
			index: ScheduleTaskIndex,
		},
		/// A call is authorized.
		AuthorizedCall {
			hash: T::Hash,
			caller: Option<T::AccountId>,
		},
		/// An authorized call was removed.
		RemovedAuthorizedCall { hash: T::Hash },
		/// An authorized call was triggered.
		TriggeredCallBy { hash: T::Hash, caller: T::AccountId },
	}

	#[pallet::storage]
	#[pallet::getter(fn next_task_index)]
	pub type NextTaskIndex<T: Config> = StorageValue<_, ScheduleTaskIndex, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn saved_calls)]
	pub type SavedCalls<T: Config> = StorageMap<_, Identity, T::Hash, (CallOf<T>, Option<T::AccountId>), OptionQuery>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Dispatch a dispatchable on behalf of other origin
		#[pallet::call_index(0)]
		#[pallet::weight({
			let info = call.get_dispatch_info();
			(T::WeightInfo::dispatch_as().saturating_add(info.weight), info.class)
		})]
		pub fn dispatch_as(origin: OriginFor<T>, as_origin: T::AsOriginId, call: Box<CallOf<T>>) -> DispatchResult {
			as_origin.check_dispatch_from(origin)?;

			let e = call.dispatch(as_origin.into_origin().into());

			Self::deposit_event(Event::Dispatched {
				result: e.map(|_| ()).map_err(|e| e.error),
			});
			Ok(())
		}

		/// Schedule a dispatchable to be dispatched at later block.
		/// This is the only way to dispatch a call with `DelayedOrigin`.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::schedule_dispatch_without_delay())]
		pub fn schedule_dispatch(
			origin: OriginFor<T>,
			when: DispatchTime<BlockNumberFor<T>>,
			priority: Priority,
			with_delayed_origin: bool,
			call: Box<CallOf<T>>,
		) -> DispatchResult {
			T::AuthorityConfig::check_schedule_dispatch(origin.clone(), priority)?;

			let id = NextTaskIndex::<T>::mutate(|id| -> sp_std::result::Result<ScheduleTaskIndex, DispatchError> {
				let current_id = *id;
				*id = id.checked_add(1).ok_or(ArithmeticError::Overflow)?;
				Ok(current_id)
			})?;
			let now = frame_system::Pallet::<T>::block_number();
			let delay = match when {
				DispatchTime::At(x) => x.checked_sub(&now).ok_or(ArithmeticError::Overflow)?,
				DispatchTime::After(x) => x,
			};
			let schedule_origin = if with_delayed_origin {
				let origin: <T as Config>::RuntimeOrigin = From::from(origin);
				let origin: <T as Config>::RuntimeOrigin =
					From::from(DelayedOrigin::<BlockNumberFor<T>, T::PalletsOrigin> {
						delay,
						origin: Box::new(origin.caller().clone()),
					});
				origin
			} else {
				<T as Config>::RuntimeOrigin::from(origin)
			};
			let pallets_origin = schedule_origin.caller().clone();

			T::Scheduler::schedule_named(
				Encode::encode(&(&pallets_origin, id)),
				when,
				None,
				priority,
				pallets_origin.clone(),
				*call,
			)
			.map_err(|_| Error::<T>::FailedToSchedule)?;

			Self::deposit_event(Event::Scheduled {
				origin: pallets_origin,
				index: id,
			});
			Ok(())
		}

		/// Fast track a scheduled dispatchable.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::fast_track_scheduled_dispatch())]
		pub fn fast_track_scheduled_dispatch(
			origin: OriginFor<T>,
			initial_origin: Box<T::PalletsOrigin>,
			task_id: ScheduleTaskIndex,
			when: DispatchTime<BlockNumberFor<T>>,
		) -> DispatchResult {
			let now = frame_system::Pallet::<T>::block_number();
			let new_delay = match when {
				DispatchTime::At(x) => x.checked_sub(&now).ok_or(ArithmeticError::Overflow)?,
				DispatchTime::After(x) => x,
			};
			let dispatch_at = match when {
				DispatchTime::At(x) => x,
				DispatchTime::After(x) => now.saturating_add(x),
			};

			T::AuthorityConfig::check_fast_track_schedule(origin, &initial_origin, new_delay)?;
			T::Scheduler::reschedule_named((&initial_origin, task_id).encode(), when)
				.map_err(|_| Error::<T>::FailedToFastTrack)?;

			Self::deposit_event(Event::FastTracked {
				origin: *initial_origin,
				index: task_id,
				when: dispatch_at,
			});
			Ok(())
		}

		/// Delay a scheduled dispatchable.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::delay_scheduled_dispatch())]
		pub fn delay_scheduled_dispatch(
			origin: OriginFor<T>,
			initial_origin: Box<T::PalletsOrigin>,
			task_id: ScheduleTaskIndex,
			additional_delay: BlockNumberFor<T>,
		) -> DispatchResult {
			T::AuthorityConfig::check_delay_schedule(origin, &initial_origin)?;

			T::Scheduler::reschedule_named(
				(&initial_origin, task_id).encode(),
				DispatchTime::After(additional_delay),
			)
			.map_err(|_| Error::<T>::FailedToDelay)?;

			let now = frame_system::Pallet::<T>::block_number();
			let dispatch_at = now.saturating_add(additional_delay);

			Self::deposit_event(Event::Delayed {
				origin: *initial_origin,
				index: task_id,
				when: dispatch_at,
			});
			Ok(())
		}

		/// Cancel a scheduled dispatchable.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::cancel_scheduled_dispatch())]
		pub fn cancel_scheduled_dispatch(
			origin: OriginFor<T>,
			initial_origin: Box<T::PalletsOrigin>,
			task_id: ScheduleTaskIndex,
		) -> DispatchResult {
			T::AuthorityConfig::check_cancel_schedule(origin, &initial_origin)?;
			T::Scheduler::cancel_named((&initial_origin, task_id).encode()).map_err(|_| Error::<T>::FailedToCancel)?;

			Self::deposit_event(Event::Cancelled {
				origin: *initial_origin,
				index: task_id,
			});
			Ok(())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::authorize_call())]
		pub fn authorize_call(
			origin: OriginFor<T>,
			call: Box<CallOf<T>>,
			caller: Option<T::AccountId>,
		) -> DispatchResult {
			ensure_root(origin)?;
			let hash = T::Hashing::hash_of(&call);
			SavedCalls::<T>::insert(hash, (call, caller.clone()));
			Self::deposit_event(Event::AuthorizedCall { hash, caller });
			Ok(())
		}

		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::remove_authorized_call())]
		pub fn remove_authorized_call(origin: OriginFor<T>, hash: T::Hash) -> DispatchResult {
			let root_or_signed =
				EitherOfDiverse::<EnsureRoot<T::AccountId>, EnsureSigned<T::AccountId>>::ensure_origin(origin)?;

			SavedCalls::<T>::try_mutate_exists(hash, |maybe_call| {
				let (_, maybe_caller) = maybe_call.take().ok_or(Error::<T>::CallNotAuthorized)?;
				match root_or_signed {
					Either::Left(_) => {} // root, do nothing
					Either::Right(who) => {
						// signed, ensure it's the caller
						let caller = maybe_caller.ok_or(Error::<T>::CallNotAuthorized)?;
						ensure!(who == caller, Error::<T>::CallNotAuthorized);
					}
				}
				Self::deposit_event(Event::RemovedAuthorizedCall { hash });
				Ok(())
			})
		}

		#[pallet::call_index(8)]
		#[pallet::weight((
			T::WeightInfo::trigger_call().saturating_add(*call_weight_bound),
			DispatchClass::Operational,
		))]
		pub fn trigger_call(
			origin: OriginFor<T>,
			hash: T::Hash,
			call_weight_bound: Weight,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			SavedCalls::<T>::try_mutate_exists(hash, |maybe_call| {
				let (call, maybe_caller) = maybe_call.take().ok_or(Error::<T>::CallNotAuthorized)?;
				if let Some(caller) = maybe_caller {
					ensure!(who == caller, Error::<T>::TriggerCallNotPermitted);
				}
				ensure!(
					call_weight_bound.all_gte(call.get_dispatch_info().weight),
					Error::<T>::WrongCallWeightBound
				);
				let result = call.dispatch(OriginFor::<T>::root());
				Self::deposit_event(Event::TriggeredCallBy { hash, caller: who });
				Self::deposit_event(Event::Dispatched {
					result: result.map(|_| ()).map_err(|e| e.error),
				});
				Ok(Pays::No.into())
			})
		}
	}
}
