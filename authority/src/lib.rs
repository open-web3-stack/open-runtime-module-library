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

#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode)]
pub struct DelayedOrigin<BlockNumber, Origin> {
	pub delay: BlockNumber,
	pub origin: Box<Origin>,
}

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
}

/// Origin for the authority module.
pub type Origin<T> = DelayedOrigin<<T as frame_system::Trait>::BlockNumber, <T as Trait>::PalletsOrigin>;

pub trait AuthorityConfig<Origin, PalletsOrigin> {
	fn check_schedule_dispatch(origin: Origin, priority: Priority) -> DispatchResult;
	fn check_fast_track_schedule(origin: Origin, initial_origin: &PalletsOrigin) -> DispatchResult;
	fn check_delay_schedule(origin: Origin, initial_origin: &PalletsOrigin) -> DispatchResult;
	fn check_cancel_schedule(origin: Origin, initial_origin: &PalletsOrigin) -> DispatchResult;
}

pub trait AsOriginId<Origin, PalletsOrigin> {
	fn into_origin(self) -> PalletsOrigin;
	fn check_dispatch_from(&self, origin: Origin) -> DispatchResult;
}

type CallOf<T> = <T as Trait>::Call;
pub type ScheduleTaskIndex = u32;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	type Origin: From<DelayedOrigin<Self::BlockNumber, <Self as Trait>::PalletsOrigin>>
		+ IsType<<Self as frame_system::Trait>::Origin>
		+ OriginTrait<PalletsOrigin = Self::PalletsOrigin>;

	type PalletsOrigin: Parameter + Into<<Self as frame_system::Trait>::Origin>;

	type Call: Parameter
		+ Dispatchable<Origin = <Self as frame_system::Trait>::Origin, PostInfo = PostDispatchInfo>
		+ GetDispatchInfo;

	/// The Scheduler.
	type Scheduler: ScheduleNamed<Self::BlockNumber, <Self as Trait>::Call, Self::PalletsOrigin>;

	type AsOriginId: Parameter + AsOriginId<<Self as frame_system::Trait>::Origin, Self::PalletsOrigin>;

	type AuthorityConfig: AuthorityConfig<<Self as frame_system::Trait>::Origin, Self::PalletsOrigin>;
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
				pallets_origin,
				*call,
			).map_err(|_| Error::<T>::FailedToSchedule)?;
		}

		#[weight = 0]
		pub fn fast_track_scheduled_dispatch(
			origin,
			initial_origin: T::PalletsOrigin,
			task_id: ScheduleTaskIndex,
			when: DispatchTime<T::BlockNumber>,
		) {
			T::AuthorityConfig::check_fast_track_schedule(origin, &initial_origin)?;

			let now = frame_system::Module::<T>::block_number();

			let when = match when {
				DispatchTime::At(x) => x,
				DispatchTime::After(x) => now.saturating_add(x)
			};

			// TODO: depends https://github.com/paritytech/substrate/issues/6774

			Self::deposit_event(RawEvent::FastTracked(initial_origin, task_id, when));
		}

		#[weight = 0]
		pub fn delay_scheduled_dispatch(
			origin,
			initial_origin: T::PalletsOrigin,
			task_id: ScheduleTaskIndex,
			when: DispatchTime<T::BlockNumber>,
		) {
			T::AuthorityConfig::check_delay_schedule(origin, &initial_origin)?;

			let now = frame_system::Module::<T>::block_number();

			let when = match when {
				DispatchTime::At(x) => x,
				DispatchTime::After(x) => now.saturating_add(x)
			};

			// TODO: depends https://github.com/paritytech/substrate/issues/6774

			Self::deposit_event(RawEvent::Delayed(initial_origin, task_id, when));
		}

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
