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
	BoundedVec,
};
use frame_system::pallet_prelude::*;
use parity_scale_codec::MaxEncodedLen;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{SaturatedConversion, Saturating},
	DispatchResult, RuntimeDebug,
};

mod default_weight;
mod mock;
mod tests;

/// Gradually update a value stored at `key` to `target_value`,
/// change `per_block` * `T::UpdateFrequency` per `T::UpdateFrequency`
/// blocks.
#[derive(Encode, Decode, Clone, Eq, PartialEq, MaxEncodedLen, RuntimeDebug, TypeInfo)]
pub struct GraduallyUpdate<Key, Value> {
	pub key: Key,
	pub target_value: Value,
	pub per_block: Value,
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

	pub(crate) type StorageKeyBytes<T> = BoundedVec<u8, <T as Config>::MaxStorageKeyBytes>;
	pub(crate) type StorageValueBytes<T> = BoundedVec<u8, <T as Config>::MaxStorageValueBytes>;

	type GraduallyUpdateOf<T> = GraduallyUpdate<StorageKeyBytes<T>, StorageValueBytes<T>>;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The frequency of updating values between blocks
		#[pallet::constant]
		type UpdateFrequency: Get<BlockNumberFor<Self>>;

		/// The origin that can schedule an update
		type DispatchOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;

		/// Maximum active gradual updates
		type MaxGraduallyUpdate: Get<u32>;

		/// Maximum size of storage key
		type MaxStorageKeyBytes: Get<u32>;

		/// Maximum size of storage value
		type MaxStorageValueBytes: Get<u32>;
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
		/// Maximum updates exceeded
		MaxGraduallyUpdateExceeded,
		/// Maximum key size exceeded
		MaxStorageKeyBytesExceeded,
		/// Maximum value size exceeded
		MaxStorageValueBytesExceeded,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Gradually update added.
		GraduallyUpdateAdded {
			key: StorageKeyBytes<T>,
			per_block: StorageValueBytes<T>,
			target_value: StorageValueBytes<T>,
		},
		/// Gradually update cancelled.
		GraduallyUpdateCancelled { key: StorageKeyBytes<T> },
		/// Gradually update applied.
		Updated {
			block_number: BlockNumberFor<T>,
			key: StorageKeyBytes<T>,
			target_value: StorageValueBytes<T>,
		},
	}

	/// All the on-going updates
	#[pallet::storage]
	#[pallet::getter(fn gradually_updates)]
	pub(crate) type GraduallyUpdates<T: Config> =
		StorageValue<_, BoundedVec<GraduallyUpdateOf<T>, T::MaxGraduallyUpdate>, ValueQuery>;

	/// The last updated block number
	#[pallet::storage]
	#[pallet::getter(fn last_updated_at)]
	pub(crate) type LastUpdatedAt<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// `on_initialize` to return the weight used in `on_finalize`.
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			if Self::_need_update(now) {
				T::WeightInfo::on_finalize(GraduallyUpdates::<T>::get().len() as u32)
			} else {
				Weight::zero()
			}
		}

		/// Update gradually_update to adjust numeric parameter.
		fn on_finalize(now: BlockNumberFor<T>) {
			Self::_on_finalize(now);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add gradually_update to adjust numeric parameter.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::gradually_update())]
		pub fn gradually_update(origin: OriginFor<T>, update: GraduallyUpdateOf<T>) -> DispatchResult {
			T::DispatchOrigin::try_origin(origin).map(|_| ()).or_else(ensure_root)?;

			// Support max value is u128, ensure per_block and target_value <= 16 bytes.
			ensure!(
				update.per_block.len() == update.target_value.len() && update.per_block.len() <= 16,
				Error::<T>::InvalidPerBlockOrTargetValue
			);

			if storage::unhashed::exists(&update.key) {
				let current_value = storage::unhashed::get::<StorageValueBytes<T>>(&update.key).unwrap();
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

				gradually_updates
					.try_push(update.clone())
					.map_err(|_| Error::<T>::MaxGraduallyUpdateExceeded)?;

				Ok(())
			})?;

			Self::deposit_event(Event::GraduallyUpdateAdded {
				key: update.key,
				per_block: update.per_block,
				target_value: update.target_value,
			});
			Ok(())
		}

		/// Cancel gradually_update to adjust numeric parameter.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::cancel_gradually_update())]
		pub fn cancel_gradually_update(origin: OriginFor<T>, key: StorageKeyBytes<T>) -> DispatchResult {
			T::DispatchOrigin::try_origin(origin).map(|_| ()).or_else(ensure_root)?;

			GraduallyUpdates::<T>::try_mutate(|gradually_updates| -> DispatchResult {
				let old_len = gradually_updates.len();
				gradually_updates.retain(|item| item.key != key);

				ensure!(gradually_updates.len() != old_len, Error::<T>::GraduallyUpdateNotFound);

				Ok(())
			})?;

			Self::deposit_event(Event::GraduallyUpdateCancelled { key });
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn _need_update(now: BlockNumberFor<T>) -> bool {
		now >= Self::last_updated_at().saturating_add(T::UpdateFrequency::get())
	}

	fn _on_finalize(now: BlockNumberFor<T>) {
		if !Self::_need_update(now) {
			return;
		}

		let mut gradually_updates = GraduallyUpdates::<T>::get();
		let initial_count = gradually_updates.len();

		gradually_updates.retain(|update| {
			let mut keep = true;
			let current_value = storage::unhashed::get::<StorageValueBytes<T>>(&update.key).unwrap_or_default();
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

			let bounded_value: StorageValueBytes<T> = value.to_vec().try_into().unwrap();

			Self::deposit_event(Event::Updated {
				block_number: now,
				key: update.key.clone(),
				target_value: bounded_value,
			});

			keep
		});

		// gradually_update has finished. Remove it from GraduallyUpdates.
		if gradually_updates.len() < initial_count {
			GraduallyUpdates::<T>::put(gradually_updates);
		}

		LastUpdatedAt::<T>::put(now);
	}

	#[allow(clippy::ptr_arg)]
	fn convert_vec_to_u8(input: &StorageValueBytes<T>) -> [u8; 16] {
		let mut array: [u8; 16] = [0; 16];
		for (i, v) in input.iter().enumerate() {
			array[i] = *v;
		}
		array
	}
}
