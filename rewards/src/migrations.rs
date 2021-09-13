use super::*;

/// The Reward Pool Info.
#[derive(Clone, Encode, Decode, PartialEq, Eq, RuntimeDebug, Default, MaxEncodedLen)]
pub struct PoolInfoV0<Share: HasCompact, Balance: HasCompact> {
	/// Total shares amount
	#[codec(compact)]
	pub total_shares: Share,
	/// Total rewards amount
	#[codec(compact)]
	pub total_rewards: Balance,
	/// Total withdrawn rewards amount
	#[codec(compact)]
	pub total_withdrawn_rewards: Balance,
}

pub fn migrate_to_multi_currency_reward<T: Config>(
	get_reward_currency: Box<dyn Fn(&T::PoolId) -> T::CurrencyId>,
) -> Weight {
	let mut reads_writes = 0;

	// migrate `Pools` to `PoolInfos`
	for (old_pool_id, old_pool_info) in Pools::<T>::drain() {
		if let Some(pool_id) = T::PoolIdConvertor::convert(old_pool_id) {
			PoolInfos::<T>::mutate(&pool_id, |pool_info| {
				let currency_id = get_reward_currency(&pool_id);
				let new_rewards = (old_pool_info.total_rewards, old_pool_info.total_withdrawn_rewards);

				pool_info.total_shares = old_pool_info.total_shares;
				pool_info
					.rewards
					.entry(currency_id)
					.and_modify(|v| {
						*v = new_rewards;
					})
					.or_insert(new_rewards);
			});
		}
	}

	// migrate `ShareAndWithdrawnReward` to `SharesAndWithdrawnRewards`
	for (old_pool_id, who, (share_amount, withdrawn_reward)) in ShareAndWithdrawnReward::<T>::drain() {
		if let Some(pool_id) = T::PoolIdConvertor::convert(old_pool_id) {
			SharesAndWithdrawnRewards::<T>::mutate(&pool_id, who, |(share, multi_withdrawn)| {
				let currency_id = get_reward_currency(&pool_id);

				*share = share_amount;
				*multi_withdrawn
					.entry(currency_id)
					.and_modify(|v| {
						*v = withdrawn_reward;
					})
					.or_insert(withdrawn_reward);
			});
		}
	}

	// Return the weight consumed by the migration.
	T::DbWeight::get().reads_writes(reads_writes, reads_writes)
}

// #[test]
// fn migrate_to_multi_currency_reward_works() {
// 	use super::mock::*;

// 	ExtBuilder::default().build().execute_with(|| {
// 		PoolInfoV0 {
// 			total_shares: 100u64,
// 			total_rewards: 1000u64,
// 			total_withdrawn_rewards: 500u64,
// 		}
// 		.using_encoded(|data| {
// 			let key = Pools::<Runtime>::hashed_key_for(&DOT_POOL);
// 			sp_io::storage::set(&key[..], data);
// 		});

// 		(100u64, 500u64).using_encoded(|data| {
// 			let key = ShareAndWithdrawnReward::<Runtime>::hashed_key_for(&DOT_POOL,
// &ALICE); 			sp_io::storage::set(&key[..], data);
// 		});

// 		let weight = migrate_to_multi_currency_reward::<Runtime>(Box::new(|_|
// STABLE_COIN)); 		assert_eq!(weight, 250_000_000);

// 		assert_eq!(
// 			Pools::<Runtime>::get(&DOT_POOL),
// 			PoolInfo {
// 				total_shares: 100,
// 				rewards: vec![(STABLE_COIN, (1000, 500))].into_iter().collect(),
// 			}
// 		);
// 		assert_eq!(
// 			ShareAndWithdrawnReward::<Runtime>::get(&DOT_POOL, &ALICE),
// 			(100, vec![(STABLE_COIN, 500u64)].into_iter().collect()),
// 		);
// 	});
// }
