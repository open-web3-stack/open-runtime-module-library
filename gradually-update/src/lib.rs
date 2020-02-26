#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, storage, traits::Get};
use frame_system::ensure_root;

use sp_runtime::RuntimeDebug;
use sp_state_machine::{StorageKey, StorageValue};

mod mock;
mod test;

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
	pub enum Event<T> where
	<T as frame_system::Trait>::BlockNumber,
	GraduallyUpdate = self::GraduallyUpdate,
	{
		/// gradually_update Success
		GraduallyUpdate(GraduallyUpdate),
		CancelGraduallyUpdate(GraduallyUpdate),
		GraduallyUpdateBlockNumber(BlockNumber,GraduallyUpdate),
	}
);

decl_error! {
	/// Error for gradually-updatemodule.
	pub enum Error for Module<T: Trait> {
		InvalidTargetValue,
		GraduallyUpdateHasExisted,
		CancelGradullyUpdateNotExisted,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		//fn deposit_event() = default;

		const UpdateFrequency: T::BlockNumber = T::UpdateFrequency::get();

		///
		pub fn gradually_update(origin, update: GraduallyUpdate) {
			ensure_root(origin)?;

			//TODO
			//value = storage::unhashed::get::<StorageValue>(&update.key);
			//if update.target_value > value
			//ensure!(value + update.per_block <= update.target_value, Error::<T>::InvalidTargetValue);
			ensure!(!GraduallyUpdates::get().contains(&update), Error::<T>::GraduallyUpdateHasExisted);

			GraduallyUpdates::get().push(update);
		}

		///
		pub fn cancel_gradually_update(origin, key: StorageKey) {
			ensure_root(origin)?;

			let cancel_update: Vec<GraduallyUpdate> = GraduallyUpdates::get()
				.into_iter()
				.filter(|item| item.key != key)
				.collect();

			ensure!(GraduallyUpdates::get().len() - cancel_update.len() != 1, Error::<T>::CancelGradullyUpdateNotExisted);

			GraduallyUpdates::put(cancel_update);
		}

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
		for (i, update) in gradually_updates.iter().enumerate() {
			//let a = Decode::decode::<[u8]>(update.target_value.as_mut_slice());;
			//if let current_value = storage::unhashed::get::<u32>(&update.key) {
			//	if current_value > update.target_value {
			//		current_value -= (update.per_block * T::UpdateFrequency::get()).max(update.target_value);
			//	} else {
			//		current_value += (update.per_block * T::UpdateFrequency::get().into()).min(update.target_value);
			//	}
			//}

			//if current_value == update.target_value {
			//	gradually_updates.remove(i);
			//}
			//storage::unhashed::put(&update.key, &current_value);
		}
		GraduallyUpdates::put(gradually_updates);
		GraduallyUpdateBlockNumber::<T>::put(now);
	}
}
