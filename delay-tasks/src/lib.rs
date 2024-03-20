#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	pallet_prelude::*,
	traits::{
		schedule::{v1::Named as ScheduleNamed, DispatchTime},
		OriginTrait,
	},
	weights::Weight,
};
use frame_system::pallet_prelude::*;
use orml_traits::{
	task::{DelayTaskHooks, DelayTasksManager, DispatchableTask},
	MultiCurrency, NamedMultiReservableCurrency,
};
use parity_scale_codec::FullCodec;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{CheckedAdd, Convert, Zero},
	ArithmeticError,
};
use sp_std::fmt::Debug;
use sp_std::marker::PhantomData;
use xcm::v4::prelude::*;

pub use module::*;

mod mock;
mod tests;

pub const DELAY_TASK_ID: [u8; 8] = *b"orml/dts";

/// A delayed origin. Can only be dispatched via `dispatch_as` with a delay.
#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct DelayedExecuteOrigin;

pub struct EnsureDelayed;
impl<O: Into<Result<DelayedExecuteOrigin, O>> + From<DelayedExecuteOrigin>> EnsureOrigin<O> for EnsureDelayed {
	type Success = ();
	fn try_origin(o: O) -> Result<Self::Success, O> {
		o.into().map(|_| ())
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<O, ()> {
		Ok(O::from(DelayedExecuteOrigin))
	}
}

#[frame_support::pallet]
pub mod module {
	use super::*;

	type Nonce = u64;

	/// Origin for the delay tasks module.
	#[pallet::origin]
	pub type Origin = DelayedExecuteOrigin;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type RuntimeCall: Parameter + From<Call<Self>>;

		/// The outer origin type.
		type RuntimeOrigin: From<DelayedExecuteOrigin>
			+ From<<Self as frame_system::Config>::RuntimeOrigin>
			+ OriginTrait<PalletsOrigin = Self::PalletsOrigin>;

		/// The caller origin, overarching type of all pallets origins.
		type PalletsOrigin: Parameter + Into<<Self as frame_system::Config>::RuntimeOrigin>;

		type DelayOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

		type GovernanceOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

		type Task: DispatchableTask + FullCodec + Debug + Clone + PartialEq + TypeInfo;

		/// The Scheduler.
		type Scheduler: ScheduleNamed<BlockNumberFor<Self>, <Self as Config>::RuntimeCall, Self::PalletsOrigin>;

		type DelayTaskHooks: DelayTaskHooks<Self::Task>;

		/// Convert `Location` to `CurrencyId`.
		type CurrencyIdConvert: Convert<
			Location,
			Option<<Self::Currency as MultiCurrency<Self::AccountId>>::CurrencyId>,
		>;

		type Currency: NamedMultiReservableCurrency<Self::AccountId>;

		type ReserveId: Get<<Self::Currency as NamedMultiReservableCurrency<Self::AccountId>>::ReserveIdentifier>;
	}

	#[pallet::error]
	pub enum Error<T> {
		InvalidDelayBlock,
		InvalidId,
		FailedToSchedule,
		AssetIndexNonExistent,
		AssetConvertFailed,
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
		DelayedTaskCanceled {
			id: Nonce,
		},
		DelayedTaskStuck {
			id: Nonce,
			error: DispatchError,
		},
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::storage]
	#[pallet::getter(fn next_delayed_task_id)]
	pub type NextDelayedTaskId<T: Config> = StorageValue<_, Nonce, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn delayed_tasks)]
	pub type DelayedTasks<T: Config> = StorageMap<_, Twox64Concat, Nonce, (T::Task, BlockNumberFor<T>), OptionQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::zero())]
		pub fn delayed_execute(origin: OriginFor<T>, id: Nonce) -> DispatchResult {
			T::DelayOrigin::ensure_origin(origin)?;

			let (delayed_task, _) = DelayedTasks::<T>::get(id).ok_or(Error::<T>::InvalidId)?;

			// pre delayed execute
			if let Err(error) = T::DelayTaskHooks::pre_delayed_execute(&delayed_task) {
				Self::deposit_event(Event::<T>::DelayedTaskStuck { id, error });
			} else {
				let execute_result = delayed_task.dispatch(Weight::zero());

				DelayedTasks::<T>::remove(id);
				Self::deposit_event(Event::<T>::DelayedTaskExecuted {
					id,
					result: execute_result.result,
				});
			}

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(Weight::zero())]
		pub fn reschedule_delay_task(
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

				T::Scheduler::reschedule_named((&DELAY_TASK_ID, id).encode(), DispatchTime::At(new_execute_block))
					.map_err(|_| Error::<T>::FailedToSchedule)?;

				*execute_block = new_execute_block;

				Self::deposit_event(Event::<T>::DelayedTaskReDelayed {
					id,
					execute_block: new_execute_block,
				});
				Ok(())
			})?;

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(Weight::zero())]
		pub fn cancel_delayed_task(origin: OriginFor<T>, id: Nonce, skip_pre_cancel: bool) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			let (task, execute_block) = DelayedTasks::<T>::take(id).ok_or(Error::<T>::InvalidId)?;

			if !skip_pre_cancel {
				T::DelayTaskHooks::pre_cancel(&task)?;
			}

			if frame_system::Pallet::<T>::block_number() < execute_block {
				// if now < execute_block, need cancel scheduler
				T::Scheduler::cancel_named((&DELAY_TASK_ID, id).encode()).map_err(|_| Error::<T>::FailedToSchedule)?;
			}

			Self::deposit_event(Event::<T>::DelayedTaskCanceled { id });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
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
			let execute_block = frame_system::Pallet::<T>::block_number()
				.checked_add(&delay_blocks)
				.ok_or(ArithmeticError::Overflow)?;

			// pre schedule delay task
			T::DelayTaskHooks::pre_delay(&task)?;

			let id = Self::get_next_delayed_task_id()?;
			let delayed_origin: <T as Config>::RuntimeOrigin = From::from(DelayedExecuteOrigin);
			let pallets_origin = delayed_origin.caller().clone();

			T::Scheduler::schedule_named(
				(&DELAY_TASK_ID, id).encode(),
				DispatchTime::At(execute_block),
				None,
				Zero::zero(),
				pallets_origin,
				<T as Config>::RuntimeCall::from(Call::<T>::delayed_execute { id }),
			)
			.map_err(|_| Error::<T>::FailedToSchedule)?;

			DelayedTasks::<T>::insert(id, (&task, execute_block));

			Self::deposit_event(Event::<T>::DelayedTaskAdded {
				id,
				task,
				execute_block,
			});
			Ok(())
		}
	}

	pub struct DelayedXtokensTaskHooks<T>(PhantomData<T>);
	impl<T: Config + orml_xtokens::Config> DelayTaskHooks<orml_xtokens::XtokensTask<T>> for DelayedXtokensTaskHooks<T>
	where
		<T as Config>::Currency: MultiCurrency<
			T::AccountId,
			CurrencyId = <T as orml_xtokens::Config>::CurrencyId,
			Balance = <T as orml_xtokens::Config>::Balance,
		>,
	{
		fn pre_delay(task: &orml_xtokens::XtokensTask<T>) -> DispatchResult {
			match task {
				orml_xtokens::XtokensTask::<T>::TransferAssets { who, assets, .. } => {
					let asset_len = assets.len();
					for i in 0..asset_len {
						let asset = assets.get(i).ok_or(Error::<T>::AssetIndexNonExistent)?;
						let currency_id: <T::Currency as MultiCurrency<T::AccountId>>::CurrencyId =
							<T as Config>::CurrencyIdConvert::convert(asset.id.0.clone())
								.ok_or(Error::<T>::AssetConvertFailed)?;
						let amount: T::Balance = match asset.fun {
							Fungibility::Fungible(amount) => {
								amount.try_into().map_err(|_| Error::<T>::AssetConvertFailed)?
							}
							Fungibility::NonFungible(_) => return Err(Error::<T>::AssetConvertFailed.into()),
						};

						T::Currency::reserve_named(&T::ReserveId::get(), currency_id, who, amount)?;
					}
				}
			}

			Ok(())
		}

		fn pre_delayed_execute(task: &orml_xtokens::XtokensTask<T>) -> DispatchResult {
			match task {
				orml_xtokens::XtokensTask::<T>::TransferAssets { who, assets, .. } => {
					let asset_len = assets.len();
					for i in 0..asset_len {
						let asset = assets.get(i).ok_or(Error::<T>::AssetIndexNonExistent)?;
						let currency_id: <T::Currency as MultiCurrency<T::AccountId>>::CurrencyId =
							<T as Config>::CurrencyIdConvert::convert(asset.id.0.clone())
								.ok_or(Error::<T>::AssetConvertFailed)?;
						let amount: T::Balance = match asset.fun {
							Fungibility::Fungible(amount) => {
								amount.try_into().map_err(|_| Error::<T>::AssetConvertFailed)?
							}
							Fungibility::NonFungible(_) => return Err(Error::<T>::AssetConvertFailed.into()),
						};

						T::Currency::unreserve_named(&T::ReserveId::get(), currency_id, who, amount);
					}
				}
			}

			Ok(())
		}

		fn pre_cancel(task: &orml_xtokens::XtokensTask<T>) -> DispatchResult {
			Self::pre_delayed_execute(task)
		}
	}
}
