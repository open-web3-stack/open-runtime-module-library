#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, storage, traits::Get};
use frame_system::{self as system, ensure_root};

use sp_runtime::{traits::SaturatedConversion, RuntimeDebug};
use sp_std::prelude::Vec;

mod mock;
mod tests;

type StorageKey = Vec<u8>;
type StorageValue = Vec<u8>;

#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct GraduallyUpdate {
	key: StorageKey,
	target_value: StorageValue,
	per_block: StorageValue,
}

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
	type UpdateFrequency: Get<Self::BlockNumber>;
}

decl_storage! {
	trait Store for Module<T: Trait> as GraduallyUpdate {
		pub GraduallyUpdates get(fn gradually_updates): Vec<GraduallyUpdate>;
		pub GraduallyUpdateBlockNumber get(fn gradually_update_block_number): T::BlockNumber;
	}
}

decl_event!(
	/// Event for gradually-update module.
	pub enum Event<T> where
	<T as frame_system::Trait>::BlockNumber,
	{
		/// Add gradually_update success (key, per_block, target_value)
		GraduallyUpdate(StorageKey, StorageValue, StorageValue),
		/// Cancel gradually_update success (key)
		CancelGraduallyUpdate(StorageKey),
		/// Update gradually_update success (blocknum, key, target_value)
		GraduallyUpdateBlockNumber(BlockNumber, StorageKey, StorageValue),
	}
);

decl_error! {
	/// Error for gradually-update module.
	pub enum Error for Module<T: Trait> {
		InvalidPerBlockOrTargetValue,
		InvalidTargetValue,
		GraduallyUpdateHasExisted,
		CancelGradullyUpdateNotExisted,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		const UpdateFrequency: T::BlockNumber = T::UpdateFrequency::get();

		/// Add gradually_update to adjust numeric parameter. This is a root call.
		#[weight = 0]
		pub fn gradually_update(origin, update: GraduallyUpdate) {
			ensure_root(origin)?;

			// Support max value is u128, ensure per_block and target_value <= 16 bytes.
			ensure!(update.per_block.len() == update.target_value.len() && update.per_block.len() <= 16, Error::<T>::InvalidPerBlockOrTargetValue);

			if storage::unhashed::exists(&update.key) {
				let current_value = storage::unhashed::get::<StorageValue>(&update.key).unwrap();
				ensure!(current_value.len() == update.target_value.len(), Error::<T>::InvalidTargetValue);
			}

			let mut gradually_updates = GraduallyUpdates::get();
			ensure!(!gradually_updates.contains(&update), Error::<T>::GraduallyUpdateHasExisted);

			gradually_updates.push(update.clone());
			GraduallyUpdates::put(gradually_updates);

			Self::deposit_event(RawEvent::GraduallyUpdate(update.key, update.per_block, update.target_value));
		}

		/// Cancel gradually_update to adjust numeric parameter. This is a root call.
		#[weight = 0]
		pub fn cancel_gradually_update(origin, key: StorageKey) {
			ensure_root(origin)?;

			let gradually_updates: Vec<GraduallyUpdate> = GraduallyUpdates::get()
				.into_iter()
				.filter(|item| item.key != key)
				.collect();

			ensure!(GraduallyUpdates::decode_len().unwrap_or_default() - gradually_updates.len() == 1, Error::<T>::CancelGradullyUpdateNotExisted);
			GraduallyUpdates::put(gradually_updates);

			Self::deposit_event(RawEvent::CancelGraduallyUpdate(key));
		}

		/// Update gradually_update to adjust numeric parameter.
		fn on_finalize(now: T::BlockNumber) {
			Self::_on_finalize(now);
		}
	}
}

impl<T: Trait> Module<T> {
	fn _on_finalize(now: T::BlockNumber) {
		if now < GraduallyUpdateBlockNumber::<T>::get() + T::UpdateFrequency::get() {
			return;
		}

		let mut gradually_updates = GraduallyUpdates::get();
		for (i, update) in gradually_updates.clone().iter().enumerate() {
			let current_value = storage::unhashed::get::<StorageValue>(&update.key).unwrap_or_default();
			let current_value_u128 = u128::from_le_bytes(Self::convert_vec_to_u8(&current_value));

			let frequency_u128: u128 = T::UpdateFrequency::get().saturated_into();

			let step = u128::from_le_bytes(Self::convert_vec_to_u8(&update.per_block));
			let step_u128 = step.checked_mul(frequency_u128).unwrap();

			let target_u128 = u128::from_le_bytes(Self::convert_vec_to_u8(&update.target_value));

			let new_value_u128: u128;
			if current_value_u128 > target_u128 {
				new_value_u128 = (current_value_u128.checked_sub(step_u128).unwrap()).max(target_u128);
			} else {
				new_value_u128 = (current_value_u128.checked_add(step_u128).unwrap()).min(target_u128);
			}

			// current_value equal target_value, remove gradually_update
			if new_value_u128 == target_u128 {
				gradually_updates.remove(i);
			}

			let mut value = new_value_u128.encode();
			value.truncate(update.target_value.len());

			storage::unhashed::put(&update.key, &value);

			Self::deposit_event(RawEvent::GraduallyUpdateBlockNumber(now, update.key.clone(), value));
		}

		// gradually_update has finished. Remove it from GraduallyUpdates.
		if gradually_updates.len() < GraduallyUpdates::decode_len().unwrap_or_default() {
			GraduallyUpdates::put(gradually_updates);
		}

		GraduallyUpdateBlockNumber::<T>::put(now);
	}

	fn convert_vec_to_u8(input: &StorageValue) -> [u8; 16] {
		let mut array: [u8; 16] = [0; 16];
		for (i, v) in input.iter().enumerate() {
			array[i] = v.clone();
		}
		array
	}
}
