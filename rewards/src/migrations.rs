use super::*;

pub(crate) mod v1 {
	use super::*;
	use orml_traits::GetByKey;
	use sp_std::collections::btree_map::BTreeMap;

	#[cfg(feature = "try-runtime")]
	pub(crate) fn pre_migrate<T: Config>() -> Result<(), &'static str> {
		assert!(StorageVersion::<T>::get() == Releases::V0, "Storage version too high.");
		Ok(())
	}

	pub(crate) fn migrate<T: Config>() -> Weight {
		let mut reads_writes: Weight = 0;
		Pools::<T>::translate::<PoolInfoV0<T::Share, T::Balance>, _>(|pool_id, old_pool_info| {
			reads_writes += 1;
			let currency_id = T::V0Migration::get(&pool_id);

			let mut rewards = BTreeMap::new();
			rewards.insert(
				currency_id,
				(old_pool_info.total_rewards, old_pool_info.total_withdrawn_rewards),
			);

			Some(PoolInfo {
				total_shares: old_pool_info.total_shares,
				rewards,
			})
		});

		ShareAndWithdrawnReward::<T>::translate::<(T::Share, T::Balance), _>(
			|pool_id, _who, (shares, withdrawn_rewards)| {
				reads_writes += 1;
				let currency_id = T::V0Migration::get(&pool_id);

				let mut withdrawns = BTreeMap::new();
				withdrawns.insert(currency_id, withdrawn_rewards);

				Some((shares, withdrawns))
			},
		);

		// Return the weight consumed by the migration.
		T::DbWeight::get().reads_writes(reads_writes, reads_writes)
	}

	#[cfg(feature = "try-runtime")]
	pub(crate) fn post_migrate<T: Config>() -> Result<(), &'static str> {
		assert_eq!(StorageVersion::<T>::get(), Releases::V1);
		Ok(())
	}
}
