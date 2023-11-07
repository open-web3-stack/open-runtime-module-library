use crate::{Config, LocationToAssetId, Pallet, Weight};
use frame_support::pallet_prelude::*;
use frame_support::{migration::storage_key_iter, traits::OnRuntimeUpgrade, StoragePrefixedMap};

use xcm::v3::prelude::*;

pub struct Migration<T>(PhantomData<T>);
impl<T: Config> OnRuntimeUpgrade for Migration<T> {
	fn on_runtime_upgrade() -> Weight {
		let mut weight: Weight = Weight::zero();
		let onchain_version = Pallet::<T>::on_chain_storage_version();
		if onchain_version < 2 {
			let inner_weight = v2::migrate::<T>();
			weight.saturating_accrue(inner_weight);
		}
		weight
	}
}

mod v2 {
	use super::*;

	pub(crate) fn migrate<T: Config>() -> Weight {
		let mut weight: Weight = Weight::zero();
		let module_prefix = LocationToAssetId::<T>::pallet_prefix();
		let storage_prefix = LocationToAssetId::<T>::storage_prefix();

		weight.saturating_accrue(T::DbWeight::get().reads(1));
		let old_data =
			storage_key_iter::<xcm::v2::MultiLocation, T::AssetId, Twox64Concat>(module_prefix, storage_prefix)
				.drain()
				.collect::<sp_std::vec::Vec<_>>();

		for (old_key, value) in old_data {
			weight.saturating_accrue(T::DbWeight::get().writes(1));
			let new_key: MultiLocation = old_key.try_into().expect("Stored xcm::v2::MultiLocation");
			LocationToAssetId::<T>::insert(new_key, value);
		}

		StorageVersion::new(2).put::<Pallet<T>>();
		weight.saturating_accrue(T::DbWeight::get().writes(1));
		weight
	}
}
