//! # Authority
//! A module to provide features for governance including dispatch method on
//! behalf other accounts and schdule dispatchables.
//!
//! - [`Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//! - [`Module`](./struct.Module.html)
//!
//! ## Overview
//!
//! Two functionalities are provided by this module:
//! - schdule a dispatchable
//! - dispatch method with on behalf of other origins

#![cfg_attr(not(feature = "std"), no_std)]
// Disable the following three lints since they originate from an external macro
#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::boxed_local)]
#![allow(clippy::borrowed_box)]

use codec::{Decode, Encode};
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage,
	dispatch::PostDispatchInfo,
	traits::{
		schedule::{DispatchTime, Named as ScheduleNamed, Priority},
		EnsureOrigin, Get, IsType, OriginTrait,
	},
	weights::GetDispatchInfo,
	Parameter,
};
use sp_runtime::{
	traits::{CheckedSub, Dispatchable, Saturating},
	DispatchError, DispatchResult, RuntimeDebug,
};
use sp_std::prelude::*;

mod mock;
mod tests;

/// A delayed origin. Can only be dispatched via `dispatch_as` with a delay.
#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode)]
pub struct DelayedOrigin<BlockNumber, Origin> {
	/// Number of blocks that this call have been delayed.
	pub delay: BlockNumber,
	/// The initial origin.
	pub origin: Box<Origin>,
}

/// Ensure the origin have a minimum amount of delay.
pub struct EnsureDelayed<Delay, Inner, BlockNumber>(sp_std::marker::PhantomData<(Delay, Inner, BlockNumber)>);
impl<
		O: Into<Result<DelayedOrigin<BlockNumber, O>, O>> + From<DelayedOrigin<BlockNumber, O>>,
		Delay: Get<BlockNumber>,
		Inner: EnsureOrigin<O>,
		BlockNumber: PartialOrd,
	> EnsureOrigin<O> for EnsureDelayed<Delay, Inner, BlockNumber>
{
	type Success = Inner::Success;

	fn try_origin(o: O) -> Result<Self::Success, O> {
		o.into().and_then(|delayed_origin| {
			if delayed_origin.delay >= Delay::get() {
				Inner::try_origin(*delayed_origin.origin)
			} else {
				Err(*delayed_origin.origin)
			}
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn successful_origin() -> O {
		unimplemented!()
	}
}

/// Origin for the authority module.
pub type Origin<T> = DelayedOrigin<<T as frame_system::Trait>::BlockNumber, <T as Trait>::PalletsOrigin>;

/// Config for orml-authority
pub trait AuthorityConfig<Origin, PalletsOrigin, BlockNumber> {
	/// Check if the `origin` is allow to schedule a dispatchable call with a
	/// given `priority`.
	fn check_schedule_dispatch(origin: Origin, priority: Priority) -> DispatchResult;
	/// Check if the `origin` is allow to fast track a scheduled task that
	/// initially created by `initial_origin`. `new_delay` is number of blocks
	/// this dispatchable will be dispatched from now after fast track.
	fn check_fast_track_schedule(
		origin: Origin,
		initial_origin: &PalletsOrigin,
		new_delay: BlockNumber,
	) -> DispatchResult;
	/// Check if the `origin` is allow to delay a scheduled task that initially
	/// created by `inital_origin`.
	fn check_delay_schedule(origin: Origin, initial_origin: &PalletsOrigin) -> DispatchResult;
	/// Check if the `origin` is allow to cancel a scheduled task that initially
	/// created by `inital_origin`.
	fn check_cancel_schedule(origin: Origin, initial_origin: &PalletsOrigin) -> DispatchResult;
}

/// Represent an origin that can be dispatched by other origins with permission
/// check.
pub trait AsOriginId<Origin, PalletsOrigin> {
	/// Convert into `PalletsOrigin`
	fn into_origin(self) -> PalletsOrigin;
	/// Check if the `origin` is allow to dispatch call on behalf of this
	/// origin.
	fn check_dispatch_from(&self, origin: Origin) -> DispatchResult;
}

type CallOf<T> = <T as Trait>::Call;

/// The schedule task index type.
pub type ScheduleTaskIndex = u32;

/// orml-authority configuration trait.
pub trait Trait: frame_system::Trait {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	/// The outer origin type.
	type Origin: From<DelayedOrigin<Self::BlockNumber, <Self as Trait>::PalletsOrigin>>
		+ IsType<<Self as frame_system::Trait>::Origin>
		+ OriginTrait<PalletsOrigin = Self::PalletsOrigin>;

	/// The caller origin, overarching type of all pallets origins.
	type PalletsOrigin: Parameter + Into<<Self as frame_system::Trait>::Origin>;

	/// The aggregated call type.
	type Call: Parameter
		+ Dispatchable<Origin = <Self as frame_system::Trait>::Origin, PostInfo = PostDispatchInfo>
		+ GetDispatchInfo;

	/// The Scheduler.
	type Scheduler: ScheduleNamed<Self::BlockNumber, <Self as Trait>::Call, Self::PalletsOrigin>;

	/// The type represent origin that can be dispatched by other origins.
	type AsOriginId: Parameter + AsOriginId<<Self as frame_system::Trait>::Origin, Self::PalletsOrigin>;

	/// Additional permission config.
	type AuthorityConfig: AuthorityConfig<<Self as frame_system::Trait>::Origin, Self::PalletsOrigin, Self::BlockNumber>;
}

decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Calculation overflow.
		Overflow,
		/// Failed to schedule a task.
		FailedToSchedule,
		/// Failed to cancel a task.
		FailedToCancel,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as Authority {
		/// Track the next task ID.
		pub NextTaskIndex get(fn next_task_index): ScheduleTaskIndex;
	}
}

decl_event! {
	pub enum Event<T> where
		<T as Trait>::PalletsOrigin,
		<T as frame_system::Trait>::BlockNumber,
	{
		/// A call is dispatched. [result]
		Dispatched(DispatchResult),
		/// A call is scheduled. [origin, index]
		Scheduled(PalletsOrigin, ScheduleTaskIndex),
		/// A scheduled call is fast tracked. [origin, index, when]
		FastTracked(PalletsOrigin, ScheduleTaskIndex, BlockNumber),
		/// A scheduled call is delayed. [origin, index, when]
		Delayed(PalletsOrigin, ScheduleTaskIndex, BlockNumber),
		/// A scheduled call is cancelled. [origin, index]
		Cancelled(PalletsOrigin, ScheduleTaskIndex),
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: <T as frame_system::Trait>::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Dispatch a dispatchable on behalf of other origin
		#[weight = (call.get_dispatch_info().weight + 10_000, call.get_dispatch_info().class)]
		pub fn dispatch_as(
			origin,
			as_origin: T::AsOriginId,
			call: Box<CallOf<T>>,
		) {
			as_origin.check_dispatch_from(origin)?;

			let e = call.dispatch(as_origin.into_origin().into());

			Self::deposit_event(RawEvent::Dispatched(e.map(|_| ()).map_err(|e| e.error)));
		}

		/// Schdule a dispatchable to be dispatched at later block.
		/// This is the only way to dispatch a call with `DelayedOrigin`.
		#[weight = 0]
		pub fn schedule_dispatch(
			origin,
			when: DispatchTime<T::BlockNumber>,
			priority: Priority,
			with_delayed_origin: bool,
			call: Box<CallOf<T>>,
		) {
			T::AuthorityConfig::check_schedule_dispatch(origin.clone(), priority)?;

			let id = NextTaskIndex::mutate(|id| -> sp_std::result::Result<ScheduleTaskIndex, DispatchError> {
				let current_id = *id;
				*id = id.checked_add(1).ok_or(Error::<T>::Overflow)?;
				Ok(current_id)
			})?;

			let now = frame_system::Module::<T>::block_number();

			let delay = match when {
				DispatchTime::At(x) => x.checked_sub(&now).ok_or(Error::<T>::Overflow)?,
				DispatchTime::After(x) => x
			};

			let schedule_origin = if with_delayed_origin {
				let origin: <T as Trait>::Origin = From::from(origin);
				let origin: <T as Trait>::Origin = From::from(DelayedOrigin::<T::BlockNumber, T::PalletsOrigin> {
					delay,
					origin: Box::new(origin.caller().clone())
				});
				origin
			} else {
				<T as Trait>::Origin::from(origin)
			};

			let pallets_origin = schedule_origin.caller().clone();

			T::Scheduler::schedule_named(
				Encode::encode(&(&pallets_origin, id)),
				when,
				None,
				priority,
				pallets_origin.clone(),
				*call,
			).map_err(|_| Error::<T>::FailedToSchedule)?;

			Self::deposit_event(RawEvent::Scheduled(pallets_origin, id));
		}

		/// Fast track a scheduled dispatchable.
		#[weight = 0]
		pub fn fast_track_scheduled_dispatch(
			origin,
			initial_origin: T::PalletsOrigin,
			task_id: ScheduleTaskIndex,
			when: DispatchTime<T::BlockNumber>,
		) {
			let now = frame_system::Module::<T>::block_number();

			let new_delay = match when {
				DispatchTime::At(x) => x.checked_sub(&now).ok_or(Error::<T>::Overflow)?,
				DispatchTime::After(x) => x
			};

			T::AuthorityConfig::check_fast_track_schedule(origin, &initial_origin, new_delay)?;

			let now = frame_system::Module::<T>::block_number();

			let when = match when {
				DispatchTime::At(x) => x,
				DispatchTime::After(x) => now.saturating_add(x)
			};

			// TODO: depends https://github.com/paritytech/substrate/issues/6774

			Self::deposit_event(RawEvent::FastTracked(initial_origin, task_id, when));
		}

		/// Delay a scheduled dispatchable.
		#[weight = 0]
		pub fn delay_scheduled_dispatch(
			origin,
			initial_origin: T::PalletsOrigin,
			task_id: ScheduleTaskIndex,
			_additional_delay: T::BlockNumber,
		) {
			T::AuthorityConfig::check_delay_schedule(origin, &initial_origin)?;

			// TODO: depends https://github.com/paritytech/substrate/issues/6774

			Self::deposit_event(RawEvent::Delayed(initial_origin, task_id, 0.into()));
		}

		/// Cancel a scheduled dispatchable.
		#[weight = 0]
		pub fn cancel_scheduled_dispatch(
			origin,
			initial_origin: T::PalletsOrigin,
			task_id: ScheduleTaskIndex,
		) {
			T::AuthorityConfig::check_cancel_schedule(origin, &initial_origin)?;

			T::Scheduler::cancel_named((&initial_origin, task_id).encode()).map_err(|_| Error::<T>::FailedToCancel)?;

			Self::deposit_event(RawEvent::Cancelled(initial_origin, task_id));
		}
	}
}
