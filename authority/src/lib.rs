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

#![cfg_attr(not(feature = "std"), no_std)]
// Disable the following three lints since they originate from an external macro
#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::boxed_local)]
#![allow(clippy::borrowed_box)]
#![allow(clippy::unused_unit)]

use frame_support::{
	dispatch::PostDispatchInfo,
	pallet_prelude::*,
	traits::{
		schedule::{DispatchTime, Named as ScheduleNamed, Priority},
		EnsureOrigin, Get, IsType, OriginTrait,
	},
	weights::GetDispatchInfo,
};
use frame_system::pallet_prelude::*;
use sp_runtime::{
	traits::{CheckedSub, Dispatchable, Saturating},
	ArithmeticError, DispatchError, DispatchResult, RuntimeDebug,
};
use sp_std::prelude::*;

mod mock;
mod tests;
mod weights;

pub use weights::WeightInfo;

/// A delayed origin. Can only be dispatched via `dispatch_as` with a delay.
#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode)]
pub struct DelayedOrigin<BlockNumber, PalletsOrigin> {
	/// Number of blocks that this call have been delayed.
	pub delay: BlockNumber,
	/// The initial origin.
	pub origin: Box<PalletsOrigin>,
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
	fn successful_origin() -> O {
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
	/// initially created by `inital_origin`.
	fn check_delay_schedule(origin: Origin, initial_origin: &PalletsOrigin) -> DispatchResult;
	/// Check if the `origin` is allow to cancel a scheduled task that
	/// initially created by `inital_origin`.
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
	pub type Origin<T> = DelayedOrigin<<T as frame_system::Config>::BlockNumber, <T as Config>::PalletsOrigin>;
	pub(crate) type CallOf<T> = <T as Config>::Call;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The outer origin type.
		type Origin: From<DelayedOrigin<Self::BlockNumber, <Self as Config>::PalletsOrigin>>
			+ IsType<<Self as frame_system::Config>::Origin>
			+ OriginTrait<PalletsOrigin = Self::PalletsOrigin>;

		/// The caller origin, overarching type of all pallets origins.
		type PalletsOrigin: Parameter + Into<<Self as frame_system::Config>::Origin>;

		/// The aggregated call type.
		type Call: Parameter
			+ Dispatchable<Origin = <Self as frame_system::Config>::Origin, PostInfo = PostDispatchInfo>
			+ GetDispatchInfo;

		/// The Scheduler.
		type Scheduler: ScheduleNamed<Self::BlockNumber, <Self as Config>::Call, Self::PalletsOrigin>;

		/// The type represent origin that can be dispatched by other origins.
		type AsOriginId: Parameter + AsOriginId<<Self as frame_system::Config>::Origin, Self::PalletsOrigin>;

		/// Additional permission config.
		type AuthorityConfig: AuthorityConfig<
			<Self as frame_system::Config>::Origin,
			Self::PalletsOrigin,
			Self::BlockNumber,
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
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// A call is dispatched. [result]
		Dispatched(DispatchResult),
		/// A call is scheduled. [origin, index]
		Scheduled(T::PalletsOrigin, ScheduleTaskIndex),
		/// A scheduled call is fast tracked. [origin, index, when]
		FastTracked(T::PalletsOrigin, ScheduleTaskIndex, T::BlockNumber),
		/// A scheduled call is delayed. [origin, index, when]
		Delayed(T::PalletsOrigin, ScheduleTaskIndex, T::BlockNumber),
		/// A scheduled call is cancelled. [origin, index]
		Cancelled(T::PalletsOrigin, ScheduleTaskIndex),
	}

	#[pallet::storage]
	#[pallet::getter(fn next_task_index)]
	pub type NextTaskIndex<T: Config> = StorageValue<_, ScheduleTaskIndex, ValueQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Dispatch a dispatchable on behalf of other origin
		#[pallet::weight({
			let info = call.get_dispatch_info();
			(T::WeightInfo::dispatch_as().saturating_add(info.weight), info.class)
		})]
		pub fn dispatch_as(
			origin: OriginFor<T>,
			as_origin: T::AsOriginId,
			call: Box<CallOf<T>>,
		) -> DispatchResultWithPostInfo {
			as_origin.check_dispatch_from(origin)?;

			let e = call.dispatch(as_origin.into_origin().into());

			Self::deposit_event(Event::Dispatched(e.map(|_| ()).map_err(|e| e.error)));
			Ok(().into())
		}

		/// Schedule a dispatchable to be dispatched at later block.
		/// This is the only way to dispatch a call with `DelayedOrigin`.
		#[pallet::weight(T::WeightInfo::schedule_dispatch_without_delay())]
		pub fn schedule_dispatch(
			origin: OriginFor<T>,
			when: DispatchTime<T::BlockNumber>,
			priority: Priority,
			with_delayed_origin: bool,
			call: Box<CallOf<T>>,
		) -> DispatchResultWithPostInfo {
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
				let origin: <T as Config>::Origin = From::from(origin);
				let origin: <T as Config>::Origin = From::from(DelayedOrigin::<T::BlockNumber, T::PalletsOrigin> {
					delay,
					origin: Box::new(origin.caller().clone()),
				});
				origin
			} else {
				<T as Config>::Origin::from(origin)
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

			Self::deposit_event(Event::Scheduled(pallets_origin, id));
			Ok(().into())
		}

		/// Fast track a scheduled dispatchable.
		#[pallet::weight(T::WeightInfo::fast_track_scheduled_dispatch())]
		pub fn fast_track_scheduled_dispatch(
			origin: OriginFor<T>,
			initial_origin: T::PalletsOrigin,
			task_id: ScheduleTaskIndex,
			when: DispatchTime<T::BlockNumber>,
		) -> DispatchResultWithPostInfo {
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

			Self::deposit_event(Event::FastTracked(initial_origin, task_id, dispatch_at));
			Ok(().into())
		}

		/// Delay a scheduled dispatchable.
		#[pallet::weight(T::WeightInfo::delay_scheduled_dispatch())]
		pub fn delay_scheduled_dispatch(
			origin: OriginFor<T>,
			initial_origin: T::PalletsOrigin,
			task_id: ScheduleTaskIndex,
			additional_delay: T::BlockNumber,
		) -> DispatchResultWithPostInfo {
			T::AuthorityConfig::check_delay_schedule(origin, &initial_origin)?;

			T::Scheduler::reschedule_named(
				(&initial_origin, task_id).encode(),
				DispatchTime::After(additional_delay),
			)
			.map_err(|_| Error::<T>::FailedToDelay)?;

			let now = frame_system::Pallet::<T>::block_number();
			let dispatch_at = now.saturating_add(additional_delay);

			Self::deposit_event(Event::Delayed(initial_origin, task_id, dispatch_at));
			Ok(().into())
		}

		/// Cancel a scheduled dispatchable.
		#[pallet::weight(T::WeightInfo::cancel_scheduled_dispatch())]
		pub fn cancel_scheduled_dispatch(
			origin: OriginFor<T>,
			initial_origin: T::PalletsOrigin,
			task_id: ScheduleTaskIndex,
		) -> DispatchResultWithPostInfo {
			T::AuthorityConfig::check_cancel_schedule(origin, &initial_origin)?;
			T::Scheduler::cancel_named((&initial_origin, task_id).encode()).map_err(|_| Error::<T>::FailedToCancel)?;

			Self::deposit_event(Event::Cancelled(initial_origin, task_id));
			Ok(().into())
		}
	}
}
