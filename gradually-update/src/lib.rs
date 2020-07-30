//! # Gradually Update
//! A module for scheduling gradually updates to storage values.
//!
//! - [`Trait`](./trait.Trait.html)
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

use codec::{Decode, Encode};
use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure, storage,
	traits::{EnsureOrigin, Get},
};
use frame_system::ensure_root;

use sp_runtime::{traits::SaturatedConversion, DispatchResult, RuntimeDebug};
use sp_std::prelude::Vec;

mod mock;
mod tests;

type StorageKey = Vec<u8>;
type StorageValue = Vec<u8>;

/// Gradually update a value stored at `key` to `target_value`,
/// change `per_block` * `T::UpdateFrequency` per `T::UpdateFrequency` blocks.
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct GraduallyUpdate {
	/// The storage key of the value to update
	key: StorageKey,
	/// The target value
	target_value: StorageValue,
	/// The amount of the value to update per one block
	per_block: StorageValue,
}

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	/// The frequency of updating values between blocks
	type UpdateFrequency: Get<Self::BlockNumber>;
	/// The origin that can schedule an update
	type DispatchOrigin: EnsureOrigin<Self::Origin>;
}

decl_storage! {
	trait Store for Module<T: Trait> as GraduallyUpdate {
		/// All the on-going updates
		pub GraduallyUpdates get(fn gradually_updates): Vec<GraduallyUpdate>;
		/// The last updated block number
		pub LastUpdatedAt get(fn last_updated_at): T::BlockNumber;
	}
}

decl_event!(
	/// Event for gradually-update module.
	pub enum Event<T> where
	<T as frame_system::Trait>::BlockNumber,
	{
		/// Gradually update added. [key, per_block, target_value]
		GraduallyUpdateAdded(StorageKey, StorageValue, StorageValue),
		/// Gradually update cancelled. [key]
		GraduallyUpdateCancelled(StorageKey),
		/// Gradually update applied. [block_number, key, target_value]
		Updated(BlockNumber, StorageKey, StorageValue),
	}
);

decl_error! {
	/// Error for gradually-update module.
	pub enum Error for Module<T: Trait> {
		/// The `per_block` or `target_value` is invalid.
		InvalidPerBlockOrTargetValue,
		/// The `target_value` is invalid.
		InvalidTargetValue,
		/// Another update is already been scheduled for this key.
		GraduallyUpdateHasExisted,
		/// No update exists to cancel.
		GraduallyUpdateNotFound,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		const UpdateFrequency: T::BlockNumber = T::UpdateFrequency::get();

		/// Add gradually_update to adjust numeric parameter.
		#[weight = 0]
		pub fn gradually_update(origin, update: GraduallyUpdate) {
			T::DispatchOrigin::try_origin(origin).map(|_| ()).or_else(ensure_root)?;

			// Support max value is u128, ensure per_block and target_value <= 16 bytes.
			ensure!(update.per_block.len() == update.target_value.len() && update.per_block.len() <= 16, Error::<T>::InvalidPerBlockOrTargetValue);

			if storage::unhashed::exists(&update.key) {
				let current_value = storage::unhashed::get::<StorageValue>(&update.key).unwrap();
				ensure!(current_value.len() == update.target_value.len(), Error::<T>::InvalidTargetValue);
			}

			GraduallyUpdates::try_mutate(|gradually_updates| -> DispatchResult {
				ensure!(!gradually_updates.contains(&update), Error::<T>::GraduallyUpdateHasExisted);

				gradually_updates.push(update.clone());

				Ok(())
			})?;

			Self::deposit_event(RawEvent::GraduallyUpdateAdded(update.key, update.per_block, update.target_value));
		}

		/// Cancel gradually_update to adjust numeric parameter.
		#[weight = 0]
		pub fn cancel_gradually_update(origin, key: StorageKey) {
			T::DispatchOrigin::try_origin(origin).map(|_| ()).or_else(ensure_root)?;

			GraduallyUpdates::try_mutate(|gradually_updates| -> DispatchResult {
				let old_len = gradually_updates.len();
				gradually_updates.retain(|item| item.key != key);

				ensure!(gradually_updates.len() != old_len, Error::<T>::GraduallyUpdateNotFound);

				Ok(())
			})?;

			Self::deposit_event(RawEvent::GraduallyUpdateCancelled(key));
		}

		/// Update gradually_update to adjust numeric parameter.
		fn on_finalize(now: T::BlockNumber) {
			Self::_on_finalize(now);
		}
	}
}

impl<T: Trait> Module<T> {
	fn _on_finalize(now: T::BlockNumber) {
		if now < Self::last_updated_at() + T::UpdateFrequency::get() {
			return;
		}

		let mut gradually_updates = GraduallyUpdates::get();
		let initial_count = gradually_updates.len();

		gradually_updates.retain(|update| {
			let mut keep = true;
			let current_value = storage::unhashed::get::<StorageValue>(&update.key).unwrap_or_default();
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

			Self::deposit_event(RawEvent::Updated(now, update.key.clone(), value));

			keep
		});

		// gradually_update has finished. Remove it from GraduallyUpdates.
		if gradually_updates.len() < initial_count {
			GraduallyUpdates::put(gradually_updates);
		}

		LastUpdatedAt::<T>::put(now);
	}

	#[allow(clippy::ptr_arg)]
	fn convert_vec_to_u8(input: &StorageValue) -> [u8; 16] {
		let mut array: [u8; 16] = [0; 16];
		for (i, v) in input.iter().enumerate() {
			array[i] = *v;
		}
		array
	}
}
