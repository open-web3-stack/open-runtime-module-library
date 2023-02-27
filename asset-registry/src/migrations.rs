use crate::{Config, LocationToAssetId, Pallet, Weight};
use frame_support::pallet_prelude::*;
use frame_support::{migration::storage_key_iter, StoragePrefixedMap};

use xcm::v3::prelude::*;

pub mod v2 {
	use super::*;

	pub fn migrate<T: Config>() -> Weight {
		let mut weight: Weight = Weight::zero();
		let onchain_version = Pallet::<T>::on_chain_storage_version();
		if onchain_version < 2 {
			let module_prefix = LocationToAssetId::<T>::module_prefix();
			let storage_prefix = LocationToAssetId::<T>::storage_prefix();

			weight.saturating_accrue(T::DbWeight::get().reads(1));
			let old_data = storage_key_iter::<xcm::v2::MultiLocation, T::AssetId, Blake2_128Concat>(
				&module_prefix,
				storage_prefix,
			)
			.drain();

			for (old_key, value) in old_data {
				weight.saturating_accrue(T::DbWeight::get().writes(1));
				let new_key: MultiLocation = old_key.try_into().expect("Stored xcm::v2::MultiLocation");
				LocationToAssetId::<T>::insert(new_key, value);
			}

			StorageVersion::new(2).put::<Pallet<T>>();
			weight.saturating_accrue(T::DbWeight::get().writes(1));
		}
		weight
	}
}
