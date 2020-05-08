#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure,
	storage::IterableStorageDoubleMap,
	traits::Get,
	weights::{DispatchClass, GetDispatchInfo, Weight},
	Parameter,
};
use frame_system::{self as system, ensure_root, ensure_signed};
use sp_runtime::{
	traits::{CheckedAdd, Dispatchable, One},
	DispatchError, RuntimeDebug,
};
use sp_std::{prelude::*, result};

mod mock;
mod tests;

#[derive(Encode, Decode, Copy, Clone, PartialEq, Eq, RuntimeDebug)]
pub enum DelayedDispatchTime<BlockNumber> {
	At(BlockNumber),
	After(BlockNumber),
}

type DispatchId = u32;
type CallOf<T> = <T as Trait>::Call;

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type Call: Parameter + Dispatchable<Origin = <Self as frame_system::Trait>::Origin> + GetDispatchInfo;
	type MaxScheduleDispatchWeight: Get<Weight>;
}

decl_event!(
	/// Event for schedule-update module.
	pub enum Event<T> where
		<T as frame_system::Trait>::BlockNumber,
	{
		/// Add schedule dispatch success (BlockNumber, DispatchId)
		ScheduleDispatch(BlockNumber, DispatchId),
		/// Cancel deplayed dispatch success (DispatchId)
		CancelDeplayedDispatch(DispatchId),
		/// Schedule dispatch success (BlockNumber, DispatchId)
		ScheduleDispatchSuccess(BlockNumber, DispatchId),
		/// Schedule dispatch failed (DispatchId, DispatchError)
		ScheduleDispatchFail(DispatchId, DispatchError),
	}
);

decl_error! {
	/// Error for schedule-update module.
	pub enum Error for Module<T: Trait> {
		BadOrigin,
		InvalidDelayedDispatchTime,
		CannotGetNextId,
		NoPermission,
		DispatchNotExisted,
		BlockNumberOverflow,
		ExceedMaxScheduleDispatchWeight,
	}
}

decl_storage! {
	trait Store for Module<T: Trait> as ScheduleUpdate {
		pub NextId get(fn next_id): DispatchId;
		pub DelayedNormalDispatches get(fn delayed_normal_dispatches):
			double_map hasher(twox_64_concat) T::BlockNumber, hasher(twox_64_concat) DispatchId => Option<(Option<T::AccountId>, CallOf<T>, DispatchId)>;
		pub DelayedOperationalDispatches get(fn delayed_operational_dispatches):
			double_map hasher(twox_64_concat) T::BlockNumber, hasher(twox_64_concat) DispatchId => Option<(Option<T::AccountId>, CallOf<T>, DispatchId)>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		const MaxScheduleDispatchWeight: Weight = T::MaxScheduleDispatchWeight::get();

		/// Add schedule_update at block_number
		#[weight = 0]
		pub fn schedule_dispatch(origin, call: CallOf<T>, when: DelayedDispatchTime<T::BlockNumber>) {
			let who = match origin.into() {
				Ok(frame_system::RawOrigin::Root) => None,
				Ok(frame_system::RawOrigin::Signed(t)) => Some(t),
				_ => return Err(Error::<T>::BadOrigin.into())
			};

			let now = <frame_system::Module<T>>::block_number();
			let block_number = match when {
				DelayedDispatchTime::At(block_number) => {
					ensure!(block_number > now, Error::<T>::InvalidDelayedDispatchTime);
					block_number
				},
				DelayedDispatchTime::After(block_count) => {
					now.checked_add(&block_count).ok_or(Error::<T>::BlockNumberOverflow)?
				},
			};

			let id = Self::_get_next_id()?;

			match call.get_dispatch_info().class {
				DispatchClass::Normal => {
					<DelayedNormalDispatches<T>>::insert(block_number, id, (who, call, id));
				},
				DispatchClass::Operational | DispatchClass::Mandatory => {
					<DelayedOperationalDispatches<T>>::insert(block_number, id, (who, call, id));
				},
			}
			Self::deposit_event(RawEvent::ScheduleDispatch(block_number, id));
		}

		/// Cancel schedule_update
		#[weight = 0]
		pub fn cancel_deplayed_dispatch(origin, at: T::BlockNumber, id: DispatchId) {
			let is_root = ensure_root(origin.clone()).is_ok();

			if let Some((who, _, _)) = <DelayedNormalDispatches<T>>::get(at, id) {
				if !is_root {
					let w = ensure_signed(origin)?;
					ensure!(Some(w) == who, Error::<T>::NoPermission);
				}
				<DelayedNormalDispatches<T>>::remove(at, id);
			} else if let Some((who, _, _)) = <DelayedOperationalDispatches<T>>::get(at, id) {
				if !is_root {
					let w = ensure_signed(origin)?;
					ensure!(Some(w) == who, Error::<T>::NoPermission);
				}
				<DelayedOperationalDispatches<T>>::remove(at, id);
			} else {
				return Err(Error::<T>::DispatchNotExisted.into());
			}
			Self::deposit_event(RawEvent::CancelDeplayedDispatch(id));
		}

		fn on_initialize(now: T::BlockNumber) -> Weight {
			let mut weight: Weight = 0;
			let total_weight = T::MaxScheduleDispatchWeight::get();
			let next_block_number = match now.checked_add(&One::one()) {
				Some(block_number) => block_number,
				_ => return weight
			};

			// Operational calls are dispatched first and then normal calls
			// TODO: dispatches should be sorted
			let mut operational_dispatches = <DelayedOperationalDispatches<T>>::iter_prefix(now);
			let _ = operational_dispatches.try_for_each(|(_, (who, call, id))| {
				weight += call.get_dispatch_info().weight;
				if weight > total_weight {
					return Err(Error::<T>::ExceedMaxScheduleDispatchWeight);
				}

				let origin: T::Origin;
				if let Some(w) = who {
					origin = frame_system::RawOrigin::Signed(w).into();
				} else {
					origin = frame_system::RawOrigin::Root.into();
				}

				let result = call.dispatch(origin.clone());
				if let Err(e) = result {
					 Self::deposit_event(RawEvent::ScheduleDispatchFail(id, e.error));
				} else {
					 Self::deposit_event(RawEvent::ScheduleDispatchSuccess(now, id));
				}
				<DelayedOperationalDispatches<T>>::remove(now, id);
				Ok(())
			});

			let mut normal_dispatches = <DelayedNormalDispatches<T>>::iter_prefix(now);
			let _ = normal_dispatches.try_for_each(|(_, (who, call, id))| {
				weight += call.get_dispatch_info().weight;
				if weight > total_weight {
					return Err(Error::<T>::ExceedMaxScheduleDispatchWeight);
				}

				let origin: T::Origin;
				if let Some(w) = who {
					origin = frame_system::RawOrigin::Signed(w).into();
				} else {
					origin = frame_system::RawOrigin::Root.into();
				}

				let result = call.dispatch(origin.clone());
				if let Err(e) = result {
					Self::deposit_event(RawEvent::ScheduleDispatchFail(id, e.error));
				} else {
					Self::deposit_event(RawEvent::ScheduleDispatchSuccess(now, id));
				}
				<DelayedNormalDispatches<T>>::remove(now, id);
				Ok(())
			});

			// Check Call dispatch weight and ensure they don't exceed MaxScheduleDispatchWeight
			// Extra ones are moved to next block
			let operational_dispatches = <DelayedOperationalDispatches<T>>::iter_prefix(now);
			operational_dispatches.for_each(|(_, (who, call, id))| {
				<DelayedOperationalDispatches<T>>::insert(next_block_number, id, (who, call, id));
				<DelayedOperationalDispatches<T>>::remove(now, id);
			});

			let normal_dispatches = <DelayedNormalDispatches<T>>::iter_prefix(now);
			normal_dispatches.for_each(|(_, (who, call, id))| {
				<DelayedNormalDispatches<T>>::insert(next_block_number, id, (who, call, id));
				<DelayedNormalDispatches<T>>::remove(now, id);
			});

			0
		}
	}
}

impl<T: Trait> Module<T> {
	fn _get_next_id() -> result::Result<DispatchId, Error<T>> {
		let id = Self::next_id();
		let next_id = id.checked_add(One::one()).ok_or(Error::<T>::CannotGetNextId)?;
		NextId::put(next_id);
		Ok(id)
	}
}
