//! # Gradually Update
//! A module for scheduling gradually updates to storage values.
//!
//! - [`Config`](./trait.Config.html)
//! - [`Call`](./enum.Call.html)
//! - [`Module`](./struct.Module.html)
//!
//! ## Overview
//!
//! This module exposes capabilities for scheduling updates to storage values
//! gradually. This is useful to change parameter values gradually to ensure a
//! smooth transition. It is also possible to cancel an update before it reaches
//! to target value.
//!
//! NOTE: Only unsigned integer value up to 128 bits are supported. But a
//! "newtype" pattern struct that wraps an unsigned integer works too such as
//! `Permill` and `FixedU128`.

#![cfg_attr(not(feature = "std"), no_std)]
// Disable the following two lints since they originate from an external macro (namely decl_storage)
#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::unused_unit)]

use frame_support::{
	ensure,
	pallet_prelude::*,
	storage,
	traits::{EnsureOrigin, Get},
};
use frame_system::pallet_prelude::*;
use sp_runtime::{traits::SaturatedConversion, DispatchResult, RuntimeDebug};
use sp_std::prelude::Vec;

mod default_weight;
mod mock;
mod tests;

/// Gradually update a value stored at `key` to `target_value`,
/// change `per_block` * `T::UpdateFrequency` per `T::UpdateFrequency`
/// blocks.
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct GraduallyUpdate {
	/// The storage key of the value to update
	pub key: StorageKeyBytes,
	/// The target value
	pub target_value: StorageValueBytes,
	/// The amount of the value to update per one block
	pub per_block: StorageValueBytes,
}

pub use module::*;

#[frame_support::pallet]
pub mod module {
	use super::*;

	pub trait WeightInfo {
		fn gradually_update() -> Weight;
		fn cancel_gradually_update() -> Weight;
		fn on_finalize(u: u32) -> Weight;
	}

	pub(crate) type StorageKeyBytes = Vec<u8>;
	pub(crate) type StorageValueBytes = Vec<u8>;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The frequency of updating values between blocks
		#[pallet::constant]
		type UpdateFrequency: Get<Self::BlockNumber>;

		/// The origin that can schedule an update
		type DispatchOrigin: EnsureOrigin<Self::Origin>;

		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The `per_block` or `target_value` is invalid.
		InvalidPerBlockOrTargetValue,
		/// The `target_value` is invalid.
		InvalidTargetValue,
		/// Another update is already been scheduled for this key.
		GraduallyUpdateHasExisted,
		/// No update exists to cancel.
		GraduallyUpdateNotFound,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Gradually update added. [key, per_block, target_value]
		GraduallyUpdateAdded(StorageKeyBytes, StorageValueBytes, StorageValueBytes),
		/// Gradually update cancelled. [key]
		GraduallyUpdateCancelled(StorageKeyBytes),
		/// Gradually update applied. [block_number, key, target_value]
		Updated(T::BlockNumber, StorageKeyBytes, StorageValueBytes),
	}

	/// All the on-going updates
	#[pallet::storage]
	#[pallet::getter(fn gradually_updates)]
	pub(crate) type GraduallyUpdates<T: Config> = StorageValue<_, Vec<GraduallyUpdate>, ValueQuery>;

	/// The last updated block number
	#[pallet::storage]
	#[pallet::getter(fn last_updated_at)]
	pub(crate) type LastUpdatedAt<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		/// `on_initialize` to return the weight used in `on_finalize`.
		fn on_initialize(now: T::BlockNumber) -> Weight {
			if Self::_need_update(now) {
				T::WeightInfo::on_finalize(GraduallyUpdates::<T>::get().len() as u32)
			} else {
				0
			}
		}

		/// Update gradually_update to adjust numeric parameter.
		fn on_finalize(now: T::BlockNumber) {
			Self::_on_finalize(now);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add gradually_update to adjust numeric parameter.
		#[pallet::weight(T::WeightInfo::gradually_update())]
		pub fn gradually_update(origin: OriginFor<T>, update: GraduallyUpdate) -> DispatchResultWithPostInfo {
			T::DispatchOrigin::try_origin(origin).map(|_| ()).or_else(ensure_root)?;

			// Support max value is u128, ensure per_block and target_value <= 16 bytes.
			ensure!(
				update.per_block.len() == update.target_value.len() && update.per_block.len() <= 16,
				Error::<T>::InvalidPerBlockOrTargetValue
			);

			if storage::unhashed::exists(&update.key) {
				let current_value = storage::unhashed::get::<StorageValueBytes>(&update.key).unwrap();
				ensure!(
					current_value.len() == update.target_value.len(),
					Error::<T>::InvalidTargetValue
				);
			}

			GraduallyUpdates::<T>::try_mutate(|gradually_updates| -> DispatchResult {
				ensure!(
					!gradually_updates.contains(&update),
					Error::<T>::GraduallyUpdateHasExisted
				);

				gradually_updates.push(update.clone());

				Ok(())
			})?;

			Self::deposit_event(Event::GraduallyUpdateAdded(
				update.key,
				update.per_block,
				update.target_value,
			));
			Ok(().into())
		}

		/// Cancel gradually_update to adjust numeric parameter.
		#[pallet::weight(T::WeightInfo::cancel_gradually_update())]
		pub fn cancel_gradually_update(origin: OriginFor<T>, key: StorageKeyBytes) -> DispatchResultWithPostInfo {
			T::DispatchOrigin::try_origin(origin).map(|_| ()).or_else(ensure_root)?;

			GraduallyUpdates::<T>::try_mutate(|gradually_updates| -> DispatchResult {
				let old_len = gradually_updates.len();
				gradually_updates.retain(|item| item.key != key);

				ensure!(gradually_updates.len() != old_len, Error::<T>::GraduallyUpdateNotFound);

				Ok(())
			})?;

			Self::deposit_event(Event::GraduallyUpdateCancelled(key));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn _need_update(now: T::BlockNumber) -> bool {
		now >= Self::last_updated_at() + T::UpdateFrequency::get()
	}

	fn _on_finalize(now: T::BlockNumber) {
		if !Self::_need_update(now) {
			return;
		}

		let mut gradually_updates = GraduallyUpdates::<T>::get();
		let initial_count = gradually_updates.len();

		gradually_updates.retain(|update| {
			let mut keep = true;
			let current_value = storage::unhashed::get::<StorageValueBytes>(&update.key).unwrap_or_default();
			let current_value_u128 = u128::from_le_bytes(Self::convert_vec_to_u8(&current_value));

			let frequency_u128: u128 = T::UpdateFrequency::get().saturated_into();

			let step = u128::from_le_bytes(Self::convert_vec_to_u8(&update.per_block));
			let step_u128 = step.checked_mul(frequency_u128).unwrap();

			let target_u128 = u128::from_le_bytes(Self::convert_vec_to_u8(&update.target_value));

			let new_value_u128 = if current_value_u128 > target_u128 {
				(current_value_u128.checked_sub(step_u128).unwrap()).max(target_u128)
			} else {
				(current_value_u128.checked_add(step_u128).unwrap()).min(target_u128)
			};

			// current_value equal target_value, remove gradually_update
			if new_value_u128 == target_u128 {
				keep = false;
			}

			let mut value = new_value_u128.encode();
			value.truncate(update.target_value.len());

			storage::unhashed::put(&update.key, &value);

			Self::deposit_event(Event::Updated(now, update.key.clone(), value));

			keep
		});

		// gradually_update has finished. Remove it from GraduallyUpdates.
		if gradually_updates.len() < initial_count {
			GraduallyUpdates::<T>::put(gradually_updates);
		}

		LastUpdatedAt::<T>::put(now);
	}

	#[allow(clippy::ptr_arg)]
	fn convert_vec_to_u8(input: &StorageValueBytes) -> [u8; 16] {
		let mut array: [u8; 16] = [0; 16];
		for (i, v) in input.iter().enumerate() {
			array[i] = *v;
		}
		array
	}
}
