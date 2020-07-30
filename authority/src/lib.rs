#![cfg_attr(not(feature = "std"), no_std)]
// Disable the following three lints since they originate from an external macro
#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::boxed_local)]
#![allow(clippy::borrowed_box)]

use codec::{Decode, Encode};
use frame_support::{
	decl_error, decl_module, decl_storage,
	dispatch::PostDispatchInfo,
	ensure,
	traits::{EnsureOrigin, Get},
	weights::GetDispatchInfo,
	Parameter,
};
use orml_traits::{DelayedDispatchTime, DispatchId, Scheduler};
use sp_runtime::{
	traits::{CheckedAdd, CheckedSub, Dispatchable},
	RuntimeDebug,
};
use sp_std::prelude::*;

mod mock;
mod tests;

#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode)]
pub struct DelayedOrigin<BlockNumber, Origin, I> {
	pub delay: BlockNumber,
	pub origin: Origin,
	_phantom: sp_std::marker::PhantomData<I>,
}

pub struct EnsureDelayed<Delay, Inner, BlockNumber, I>(sp_std::marker::PhantomData<(Delay, Inner, BlockNumber, I)>);
impl<
		O: Into<Result<DelayedOrigin<BlockNumber, O, I>, O>> + From<DelayedOrigin<BlockNumber, O, I>>,
		Delay: Get<BlockNumber>,
		Inner: EnsureOrigin<O>,
		BlockNumber: PartialOrd,
		I,
	> EnsureOrigin<O> for EnsureDelayed<Delay, Inner, BlockNumber, I>
{
	type Success = Inner::Success;

	fn try_origin(o: O) -> Result<Self::Success, O> {
		o.into().and_then(|delayed_origin| {
			if delayed_origin.delay >= Delay::get() {
				Inner::try_origin(delayed_origin.origin)
			} else {
				Err(delayed_origin.origin)
			}
		})
	}
}

/// Origin for the authority module.
pub type Origin<T, I = DefaultInstance> = DelayedOrigin<
	<T as frame_system::Trait>::BlockNumber,
	frame_system::RawOrigin<<T as frame_system::Trait>::AccountId>,
	I,
>;
type CallOf<T, I = DefaultInstance> = <T as Trait<I>>::Call;

pub trait Trait<I: Instance = DefaultInstance>: frame_system::Trait {
	type Origin: From<DelayedOrigin<Self::BlockNumber, frame_system::RawOrigin<Self::AccountId>, I>>
		+ From<frame_system::RawOrigin<Self::AccountId>>;
	type Call: Parameter
		+ Dispatchable<Origin = <Self as frame_system::Trait>::Origin, PostInfo = PostDispatchInfo>
		+ GetDispatchInfo;
	type RootDispatchOrigin: EnsureOrigin<<Self as frame_system::Trait>::Origin>;
	type DelayedRootDispatchOrigin: EnsureOrigin<<Self as frame_system::Trait>::Origin>;
	type DelayedDispatchOrigin: EnsureOrigin<<Self as frame_system::Trait>::Origin>;
	type VetoOrigin: EnsureOrigin<<Self as frame_system::Trait>::Origin>;
	type InstantDispatchOrigin: EnsureOrigin<<Self as frame_system::Trait>::Origin>;
	type Scheduler: Scheduler<Self::BlockNumber, Origin = <Self as Trait<I>>::Origin, Call = <Self as Trait<I>>::Call>;
	type MinimumDelay: Get<Self::BlockNumber>;
	type AsOrigin: Get<<Self as frame_system::Trait>::Origin>;
}

decl_error! {
	/// Error for authority module.
	pub enum Error for Module<T: Trait<I>, I: Instance> {
		BlockNumberOverflow,
		InvalidDelayedDispatchTime,
		OriginConvertFailed,
	}
}

decl_storage! {
	trait Store for Module<T: Trait<I>, I: Instance=DefaultInstance> as Authority {}
}

decl_module! {
	pub struct Module<T: Trait<I>, I: Instance = DefaultInstance> for enum Call where origin: <T as frame_system::Trait>::Origin {
		type Error = Error<T, I>;

		const MinimumDelay: T::BlockNumber = T::MinimumDelay::get();

		#[weight = (call.get_dispatch_info().weight + 10_000, call.get_dispatch_info().class)]
		pub fn dispatch(origin, call: Box<CallOf<T, I>>) {
			T::RootDispatchOrigin::ensure_origin(origin)?;
			call.dispatch(T::AsOrigin::get()).map(|_| ()).map_err(|e| e.error)?;
		}

		#[weight = (call.get_dispatch_info().weight + 10_000, call.get_dispatch_info().class)]
		pub fn schedule_dispatch(origin, call: Box<CallOf<T, I>>, when: DelayedDispatchTime<T::BlockNumber>) {
			let now = <frame_system::Module<T>>::block_number();
			let when_block = match when {
				DelayedDispatchTime::At(at_block) => {
					ensure!(at_block > now, Error::<T, I>::InvalidDelayedDispatchTime);
					at_block
				},
				DelayedDispatchTime::After(after_block) => {
					now.checked_add(&after_block).ok_or(Error::<T, I>::BlockNumberOverflow)?
				},
			};

			if when_block >= T::MinimumDelay::get() + now {
				T::DelayedRootDispatchOrigin::ensure_origin(origin)?;
			} else {
				T::InstantDispatchOrigin::ensure_origin(origin)?;
			}

			let raw_origin: frame_system::RawOrigin<T::AccountId> = T::AsOrigin::get().into().map_err(|_| Error::<T, I>::OriginConvertFailed)?;

			// schedule call with as origin
			let _ = T::Scheduler::schedule(raw_origin.into(), *call, when);
		}

		#[weight = (call.get_dispatch_info().weight + 10_000, call.get_dispatch_info().class)]
		pub fn schedule_dispatch_delayed(origin, call: Box<CallOf<T, I>>, when: DelayedDispatchTime<T::BlockNumber>) {
			T::DelayedDispatchOrigin::ensure_origin(origin.clone())?;

			let now = <frame_system::Module<T>>::block_number();
			let delay_block = match when {
				DelayedDispatchTime::At(at_block) => {
					at_block.checked_sub(&now).ok_or(Error::<T, I>::InvalidDelayedDispatchTime)?
				},
				DelayedDispatchTime::After(after_block) => {
					ensure!(after_block.checked_add(&now).is_some(), Error::<T, I>::BlockNumberOverflow);
					after_block
				},
			};

			let raw_origin = origin.into().map_err(|_| Error::<T, I>::OriginConvertFailed)?;
			let delayed_origin = DelayedOrigin{
				delay: delay_block,
				origin: raw_origin,
				_phantom: sp_std::marker::PhantomData,
			};

			// dispatch call with DelayedOrigin
			let _ = T::Scheduler::schedule(delayed_origin.into(), *call, when);
		}

		#[weight = 0]
		pub fn veto(origin, dispatch_id: DispatchId) {
			T::VetoOrigin::ensure_origin(origin)?;
			T::Scheduler::cancel(dispatch_id);
		}
	}
}
