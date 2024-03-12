#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{pallet_prelude::*, traits::schedule::DispatchTime, transactional, weights::Weight};
use frame_system::pallet_prelude::*;
use orml_traits::delay_tasks::{DelayTasksManager, DelayedTask};
use parity_scale_codec::FullCodec;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{CheckedAdd, Zero},
	ArithmeticError,
};
use sp_std::fmt::Debug;

pub use module::*;

mod mock;
mod tests;

#[frame_support::pallet]
pub mod module {
	use super::*;

	type Nonce = u64;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type GovernanceOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

		type Task: DelayedTask + FullCodec + Debug + Clone + PartialEq + TypeInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		InvalidDelayBlock,
		InvalidId,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		DelayedTaskAdded {
			id: Nonce,
			task: T::Task,
			execute_block: BlockNumberFor<T>,
		},
		DelayedTaskExecuted {
			id: Nonce,
			result: DispatchResult,
		},
		DelayedTaskReDelayed {
			id: Nonce,
			execute_block: BlockNumberFor<T>,
		},
		DelayedTaskTryExecuteFailed {
			id: Nonce,
			error: DispatchError,
		},
		DelayedTaskCanceled {
			id: Nonce,
		},
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// `on_initialize` to return the weight used in `on_finalize`.
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			Weight::zero()
		}

		fn on_finalize(now: BlockNumberFor<T>) {
			Self::_on_finalize(now);
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn next_delayed_task_id)]
	pub type NextDelayedTaskId<T: Config> = StorageValue<_, Nonce, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn delayed_tasks)]
	pub type DelayedTasks<T: Config> = StorageMap<_, Twox64Concat, Nonce, (T::Task, BlockNumberFor<T>), OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn delayed_task_queue)]
	pub type DelayedTaskQueue<T: Config> =
		StorageDoubleMap<_, Twox64Concat, BlockNumberFor<T>, Twox64Concat, Nonce, (), OptionQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::zero())]
		pub fn reset_execute_block(
			origin: OriginFor<T>,
			id: Nonce,
			when: DispatchTime<BlockNumberFor<T>>,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			DelayedTasks::<T>::try_mutate_exists(id, |maybe_task| -> DispatchResult {
				let (_, execute_block) = maybe_task.as_mut().ok_or(Error::<T>::InvalidId)?;

				let now = frame_system::Pallet::<T>::block_number();
				let new_execute_block = match when {
					DispatchTime::At(x) => x,
					DispatchTime::After(x) => x.checked_add(&now).ok_or(ArithmeticError::Overflow)?,
				};
				ensure!(new_execute_block > now, Error::<T>::InvalidDelayBlock);

				DelayedTaskQueue::<T>::remove(*execute_block, id);
				DelayedTaskQueue::<T>::insert(new_execute_block, id, ());
				*execute_block = new_execute_block;

				Self::deposit_event(Event::<T>::DelayedTaskReDelayed {
					id,
					execute_block: new_execute_block,
				});
				Ok(())
			})?;

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(Weight::zero())]
		pub fn cancel_delayed_task(origin: OriginFor<T>, id: Nonce) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			let (delay_task, execute_block) = DelayedTasks::<T>::take(id).ok_or(Error::<T>::InvalidId)?;
			delay_task.on_cancel()?;
			DelayedTaskQueue::<T>::remove(execute_block, id);

			Self::deposit_event(Event::<T>::DelayedTaskCanceled { id });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn _on_finalize(now: BlockNumberFor<T>) {
			for (id, _) in DelayedTaskQueue::<T>::drain_prefix(now) {
				match Self::do_execute_delayed_task(id) {
					Ok(result) => {
						Self::deposit_event(Event::<T>::DelayedTaskExecuted { id, result });
					}
					Err(e) => {
						log::debug!(
							target: "delay-tasks",
							"try executing delayed task {:?} failed for: {:?}. The delayed task still exists, but needs to be canceled or reset delay block.",
							id,
							e
						);
						Self::deposit_event(Event::<T>::DelayedTaskTryExecuteFailed { id, error: e });
					}
				}
			}
		}

		#[transactional]
		pub(crate) fn do_execute_delayed_task(id: Nonce) -> sp_std::result::Result<DispatchResult, DispatchError> {
			let (delayed_task, _) = DelayedTasks::<T>::take(id).ok_or(Error::<T>::InvalidId)?;

			delayed_task.pre_delayed_execute()?;

			Ok(delayed_task.delayed_execute())
		}

		/// Retrieves the next delayed task ID from storage, and increment it by
		/// one.
		fn get_next_delayed_task_id() -> Result<Nonce, DispatchError> {
			NextDelayedTaskId::<T>::mutate(|current| -> Result<Nonce, DispatchError> {
				let id = *current;

				*current = current.checked_add(1).ok_or(ArithmeticError::Overflow)?;
				Ok(id)
			})
		}
	}

	impl<T: Config> DelayTasksManager<T::Task, BlockNumberFor<T>> for Pallet<T> {
		fn add_delay_task(task: T::Task, delay_blocks: BlockNumberFor<T>) -> DispatchResult {
			ensure!(!delay_blocks.is_zero(), Error::<T>::InvalidDelayBlock);

			task.pre_delay()?;

			let id = Self::get_next_delayed_task_id()?;
			let execute_block = frame_system::Pallet::<T>::block_number()
				.checked_add(&delay_blocks)
				.ok_or(ArithmeticError::Overflow)?;

			DelayedTasks::<T>::insert(id, (&task, execute_block));
			DelayedTaskQueue::<T>::insert(execute_block, id, ());

			Self::deposit_event(Event::<T>::DelayedTaskAdded {
				id,
				task,
				execute_block,
			});
			Ok(())
		}
	}
}
