#![cfg_attr(not(feature = "std"), no_std)]
// Disable the following two lints since they originate from an external macro (namely decl_storage)
#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::boxed_local)]
#![allow(clippy::borrowed_box)]

use frame_support::{
	decl_error, decl_module,
	dispatch::PostDispatchInfo,
	ensure,
	traits::{EnsureOrigin, Get},
	weights::GetDispatchInfo,
	Parameter,
};
use frame_system::{self as system};
use orml_traits::{DelayedDispatchTime, DispatchId, Scheduler};
use sp_runtime::{
	traits::{BadOrigin, CheckedAdd, CheckedSub, Dispatchable},
	RuntimeDebug,
};
use sp_std::prelude::*;

mod mock;
mod tests;

#[derive(PartialEq, Eq, Clone, RuntimeDebug)]
pub struct DelayedOrigin<BlockNumber, Origin> {
	pub delay: BlockNumber,
	pub origin: Origin,
}

/// Origin for the authority module.
pub type Origin<T> =
	DelayedOrigin<<T as system::Trait>::BlockNumber, system::RawOrigin<<T as system::Trait>::AccountId>>;

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
				Inner::try_origin(delayed_origin.origin)
			} else {
				Err(delayed_origin.origin)
			}
		})
	}
}

pub trait Trait: system::Trait {
	type Origin: From<DelayedOrigin<Self::BlockNumber, system::RawOrigin<Self::AccountId>>>
		+ From<system::RawOrigin<Self::AccountId>>;
	type Call: Parameter
		+ Dispatchable<Origin = <Self as system::Trait>::Origin, PostInfo = PostDispatchInfo>
		+ GetDispatchInfo;
	type RootDispatchOrigin: EnsureOrigin<<Self as system::Trait>::Origin>;
	type DelayedRootDispatchOrigin: EnsureOrigin<<Self as system::Trait>::Origin>;
	type DelayedDispatchOrigin: EnsureOrigin<<Self as system::Trait>::Origin>;
	type VetoOrigin: EnsureOrigin<<Self as system::Trait>::Origin>;
	type InstantDispatchOrigin: EnsureOrigin<<Self as system::Trait>::Origin>;
	type Scheduler: Scheduler<Self::BlockNumber, Origin = <Self as Trait>::Origin, Call = <Self as Trait>::Call>;
	type MinimumDelay: Get<Self::BlockNumber>;
}

decl_error! {
	/// Error for authority module.
	pub enum Error for Module<T: Trait> {
		BlockNumberOverflow,
		InvalidDelayedDispatchTime,
	}
}

type CallOf<T> = <T as Trait>::Call;

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: <T as system::Trait>::Origin {
		type Error = Error<T>;

		const MinimumDelay: T::BlockNumber = T::MinimumDelay::get();

		#[weight = (call.get_dispatch_info().weight + 10_000, call.get_dispatch_info().class)]
		pub fn dispatch_root(origin, call: Box<CallOf<T>>) {
			T::RootDispatchOrigin::try_origin(origin).map_err(|_| BadOrigin)?;
			call.dispatch(frame_system::RawOrigin::Root.into()).map(|_| ()).map_err(|e| e.error)?;
		}

		#[weight = (call.get_dispatch_info().weight + 10_000, call.get_dispatch_info().class)]
		pub fn schedule_dispatch_root(origin, call: Box<CallOf<T>>, when: DelayedDispatchTime<T::BlockNumber>) {
			let now = <frame_system::Module<T>>::block_number();
			let when_block = match when {
				DelayedDispatchTime::At(at_block) => {
					ensure!(at_block > now, Error::<T>::InvalidDelayedDispatchTime);
					at_block
				},
				DelayedDispatchTime::After(after_block) => {
					now.checked_add(&after_block).ok_or(Error::<T>::BlockNumberOverflow)?
				},
			};

			if when_block >= T::MinimumDelay::get() + now {
				T::DelayedRootDispatchOrigin::try_origin(origin).map_err(|_| BadOrigin)?;
			} else {
				T::InstantDispatchOrigin::try_origin(origin).map_err(|_| BadOrigin)?;
			}

			// schedule call with Root origin
			let _ = T::Scheduler::schedule(frame_system::RawOrigin::Root.into(), *call, when);
		}

		#[weight = (call.get_dispatch_info().weight + 10_000, call.get_dispatch_info().class)]
		pub fn schedule_dispatch_delayed(origin, call: Box<CallOf<T>>, when: DelayedDispatchTime<T::BlockNumber>) {
			T::DelayedDispatchOrigin::try_origin(origin.clone()).map_err(|_| BadOrigin)?;

			let now = <frame_system::Module<T>>::block_number();
			let delay_block = match when {
				DelayedDispatchTime::At(at_block) => {
					at_block.checked_sub(&now).ok_or(Error::<T>::InvalidDelayedDispatchTime)?
				},
				DelayedDispatchTime::After(after_block) => {
					ensure!(after_block.checked_add(&now).is_some(), Error::<T>::BlockNumberOverflow);
					after_block
				},
			};

			let raw_origin = origin.into().map_err(|_| BadOrigin)?;
			let delayed_origin = DelayedOrigin{
				delay: delay_block,
				origin: raw_origin,
			};

			// dispatch call with DelayedOrigin
			let _ = T::Scheduler::schedule(delayed_origin.into(), *call, when);
		}

		#[weight = 0]
		pub fn veto(origin, dispatch_id: DispatchId) {
			T::VetoOrigin::try_origin(origin).map_err(|_| BadOrigin)?;
			T::Scheduler::cancel(dispatch_id);
		}
	}
}
