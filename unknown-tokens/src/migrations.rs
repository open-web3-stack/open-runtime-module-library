use crate::{AbstractFungibleBalances, ConcreteFungibleBalances, Config, Pallet, Weight};
use frame_support::pallet_prelude::*;
use frame_support::{migration::storage_iter, traits::OnRuntimeUpgrade, ReversibleStorageHasher, StoragePrefixedMap};

use sp_std::vec::Vec;

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

		// ConcreteFungibleBalances
		let module_prefix = ConcreteFungibleBalances::<T>::module_prefix();
		let storage_prefix = ConcreteFungibleBalances::<T>::storage_prefix();

		weight.saturating_accrue(T::DbWeight::get().reads(1));

		let old_data = storage_iter::<u128>(module_prefix, storage_prefix).drain();

		for (raw_k, value) in old_data {
			let mut full_key = Vec::new();
			full_key.extend_from_slice(&raw_k);

			let mut k1_k2_material = Blake2_128Concat::reverse(&full_key);
			let k1: xcm::v2::MultiLocation =
				Decode::decode(&mut k1_k2_material).expect("Stored k1 xcm::v2::MultiLocation");

			let mut k2_material = Blake2_128Concat::reverse(k1_k2_material);
			let k2: xcm::v2::MultiLocation =
				Decode::decode(&mut k2_material).expect("Stored k2 xcm::v2::MultiLocation");

			weight.saturating_accrue(T::DbWeight::get().writes(1));
			let k1_new: MultiLocation = k1.try_into().expect("Stored k1 xcm::v2::MultiLocation");
			let k2_new: MultiLocation = k2.try_into().expect("Stored k2 xcm::v2::MultiLocation");
			ConcreteFungibleBalances::<T>::insert(k1_new, k2_new, value);
		}

		// AbstractFungibleBalances
		let module_prefix = AbstractFungibleBalances::<T>::module_prefix();
		let storage_prefix = AbstractFungibleBalances::<T>::storage_prefix();

		weight.saturating_accrue(T::DbWeight::get().reads(1));

		let old_data = storage_iter::<u128>(module_prefix, storage_prefix).drain();

		for (raw_k, value) in old_data {
			let mut full_key = Vec::new();
			full_key.extend_from_slice(&raw_k);

			let mut k1_k2_material = Blake2_128Concat::reverse(&full_key);
			let k1: xcm::v2::MultiLocation =
				Decode::decode(&mut k1_k2_material).expect("Stored k1 xcm::v2::MultiLocation");

			let mut k2_material = Blake2_128Concat::reverse(k1_k2_material);
			let k2_new: Vec<u8> = Decode::decode(&mut k2_material).expect("Stored k1 xcm::v2::MultiLocation");

			weight.saturating_accrue(T::DbWeight::get().writes(1));
			let k1_new: MultiLocation = k1.try_into().expect("Stored k1 xcm::v2::MultiLocation");
			AbstractFungibleBalances::<T>::insert(k1_new, k2_new, value);
		}

		StorageVersion::new(2).put::<Pallet<T>>();
		weight.saturating_accrue(T::DbWeight::get().writes(1));
		weight
	}
}
