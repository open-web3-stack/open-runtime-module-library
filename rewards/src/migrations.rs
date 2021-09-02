use super::*;

pub fn migrate_to_multi_currency_reward<T: Config>(get_reward_currency: impl Fn(&T::PoolId) -> T::CurrencyId) -> Weight {
	let mut reads_writes: Weight = 0;
	Pools::<T>::translate::<PoolInfoV0<T::Share, T::Balance>, _>(|pool_id, old_pool_info| {
		reads_writes += 1;
		let currency_id = get_reward_currency(&pool_id);

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
			let currency_id = get_reward_currency(&pool_id);

			let mut withdrawn = BTreeMap::new();
			withdrawn.insert(currency_id, withdrawn_rewards);

			Some((shares, withdrawn))
		},
	);

	// Return the weight consumed by the migration.
	T::DbWeight::get().reads_writes(reads_writes, reads_writes)
}
