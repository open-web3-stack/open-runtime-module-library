use super::*;
use frame_support::log;

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

// migrate storage `Pools` to `PoolInfos`
pub fn migrate_to_pool_infos<T: Config>(
	maybe_limit: Option<usize>,
	get_reward_currency: Box<dyn Fn(&T::PoolIdV0) -> T::CurrencyId>,
) -> Weight {
	let mut remove_items = 0;
	let mut insert_items = 0;

	for (old_pool_id, old_pool_info) in Pools::<T>::drain().take(maybe_limit.unwrap_or(usize::MAX)) {
		remove_items += 1;
		if let Some(pool_id) = T::PoolIdConvertor::convert(old_pool_id.clone()) {
			PoolInfos::<T>::mutate(&pool_id, |pool_info| {
				let currency_id = get_reward_currency(&old_pool_id);
				let rewards = (old_pool_info.total_rewards, old_pool_info.total_withdrawn_rewards);

				pool_info.total_shares = old_pool_info.total_shares;
				pool_info.rewards.entry(currency_id).or_insert(rewards);
			});
			insert_items += 1;
		}
	}

	log::info!(
		target: "rewards-migration",
		"migrate orml-rewards Pools: migrate {:?} items",
		remove_items,
	);

	// Return the weight consumed by the migration.
	let total_reads_writes = remove_items + insert_items;
	T::DbWeight::get().reads_writes(total_reads_writes, total_reads_writes)
}

// migrate storage `ShareAndWithdrawnReward` to `SharesAndWithdrawnRewards`
pub fn migrate_to_shares_and_withdrawn_rewards<T: Config>(
	maybe_limit: Option<usize>,
	get_reward_currency: Box<dyn Fn(&T::PoolIdV0) -> T::CurrencyId>,
) -> Weight {
	let mut remove_items = 0;
	let mut insert_items = 0;

	for (old_pool_id, who, (share_amount, withdrawn_reward)) in
		ShareAndWithdrawnReward::<T>::drain().take(maybe_limit.unwrap_or(usize::MAX))
	{
		remove_items += 1;
		if let Some(pool_id) = T::PoolIdConvertor::convert(old_pool_id.clone()) {
			SharesAndWithdrawnRewards::<T>::mutate(&pool_id, who, |(share, multi_withdrawn)| {
				let currency_id = get_reward_currency(&old_pool_id);
				*share = share_amount;
				multi_withdrawn.entry(currency_id).or_insert(withdrawn_reward);
			});
			insert_items += 1;
		}
	}

	log::info!(
		target: "rewards-migration",
		"migrate orml-rewards ShareAndWithdrawnReward: migrate {:?} items",
		remove_items,
	);

	// Return the weight consumed by the migration.
	let total_reads_writes = remove_items + insert_items;
	T::DbWeight::get().reads_writes(total_reads_writes, total_reads_writes)
}

#[test]
fn migrate_to_pool_infos_works() {
	use super::mock::*;

	ExtBuilder::default().build().execute_with(|| {
		Pools::<Runtime>::insert(
			0,
			PoolInfoV0 {
				total_shares: 200,
				total_rewards: 1000,
				total_withdrawn_rewards: 500,
			},
		);
		Pools::<Runtime>::insert(
			1,
			PoolInfoV0 {
				total_shares: 200,
				total_rewards: 2000,
				total_withdrawn_rewards: 1000,
			},
		);
		Pools::<Runtime>::insert(
			2,
			PoolInfoV0 {
				total_shares: 100,
				total_rewards: 2000,
				total_withdrawn_rewards: 500,
			},
		);
		Pools::<Runtime>::insert(
			3,
			PoolInfoV0 {
				total_shares: 100,
				total_rewards: 500,
				total_withdrawn_rewards: 100,
			},
		);
		let get_reward_currency = |pool_id: &PoolId| if pool_id % 2 == 0 { NATIVE_COIN } else { STABLE_COIN };

		assert_eq!(Pools::<Runtime>::contains_key(0), true);
		assert_eq!(Pools::<Runtime>::contains_key(1), true);
		assert_eq!(Pools::<Runtime>::contains_key(2), true);
		assert_eq!(Pools::<Runtime>::contains_key(3), true);
		assert_eq!(PoolInfos::<Runtime>::contains_key(0), false);
		assert_eq!(PoolInfos::<Runtime>::contains_key(1), false);

		assert_eq!(
			migrate_to_pool_infos::<Runtime>(None, Box::new(get_reward_currency)),
			<Runtime as frame_system::Config>::DbWeight::get().reads_writes(4 + 4, 4 + 4)
		);

		assert_eq!(Pools::<Runtime>::contains_key(0), false);
		assert_eq!(Pools::<Runtime>::contains_key(1), false);
		assert_eq!(Pools::<Runtime>::contains_key(2), false);
		assert_eq!(Pools::<Runtime>::contains_key(3), false);
		assert_eq!(PoolInfos::<Runtime>::contains_key(0), true);
		assert_eq!(PoolInfos::<Runtime>::contains_key(1), true);

		assert_eq!(
			RewardsModule::pool_infos(0),
			PoolInfo {
				total_shares: 200,
				rewards: vec![(NATIVE_COIN, (1000, 500)), (STABLE_COIN, (2000, 1000))]
					.into_iter()
					.collect()
			}
		);
		assert_eq!(
			RewardsModule::pool_infos(1),
			PoolInfo {
				total_shares: 100,
				rewards: vec![(NATIVE_COIN, (2000, 500)), (STABLE_COIN, (500, 100))]
					.into_iter()
					.collect()
			}
		);
	});
}

#[test]
fn migrate_to_shares_and_withdrawn_rewards_works() {
	use super::mock::*;

	ExtBuilder::default().build().execute_with(|| {
		ShareAndWithdrawnReward::<Runtime>::insert(0, ALICE, (100, 500));
		ShareAndWithdrawnReward::<Runtime>::insert(1, ALICE, (100, 0));
		ShareAndWithdrawnReward::<Runtime>::insert(2, ALICE, (800, 200));
		ShareAndWithdrawnReward::<Runtime>::insert(0, BOB, (500, 200));
		ShareAndWithdrawnReward::<Runtime>::insert(2, BOB, (300, 200));
		ShareAndWithdrawnReward::<Runtime>::insert(1, CAROL, (100, 5000));
		let get_reward_currency = |pool_id: &PoolId| if pool_id % 2 == 0 { NATIVE_COIN } else { STABLE_COIN };

		assert_eq!(ShareAndWithdrawnReward::<Runtime>::contains_key(0, ALICE), true);
		assert_eq!(ShareAndWithdrawnReward::<Runtime>::contains_key(1, ALICE), true);
		assert_eq!(ShareAndWithdrawnReward::<Runtime>::contains_key(2, ALICE), true);
		assert_eq!(ShareAndWithdrawnReward::<Runtime>::contains_key(0, BOB), true);
		assert_eq!(ShareAndWithdrawnReward::<Runtime>::contains_key(2, BOB), true);
		assert_eq!(ShareAndWithdrawnReward::<Runtime>::contains_key(1, CAROL), true);
		assert_eq!(SharesAndWithdrawnRewards::<Runtime>::contains_key(0, ALICE), false);
		assert_eq!(SharesAndWithdrawnRewards::<Runtime>::contains_key(1, ALICE), false);
		assert_eq!(SharesAndWithdrawnRewards::<Runtime>::contains_key(0, BOB), false);
		assert_eq!(SharesAndWithdrawnRewards::<Runtime>::contains_key(1, BOB), false);
		assert_eq!(SharesAndWithdrawnRewards::<Runtime>::contains_key(0, CAROL), false);

		assert_eq!(
			migrate_to_shares_and_withdrawn_rewards::<Runtime>(None, Box::new(get_reward_currency)),
			<Runtime as frame_system::Config>::DbWeight::get().reads_writes(6 + 6, 6 + 6)
		);

		assert_eq!(ShareAndWithdrawnReward::<Runtime>::contains_key(0, ALICE), false);
		assert_eq!(ShareAndWithdrawnReward::<Runtime>::contains_key(1, ALICE), false);
		assert_eq!(ShareAndWithdrawnReward::<Runtime>::contains_key(2, ALICE), false);
		assert_eq!(ShareAndWithdrawnReward::<Runtime>::contains_key(0, BOB), false);
		assert_eq!(ShareAndWithdrawnReward::<Runtime>::contains_key(2, BOB), false);
		assert_eq!(ShareAndWithdrawnReward::<Runtime>::contains_key(1, CAROL), false);
		assert_eq!(SharesAndWithdrawnRewards::<Runtime>::contains_key(0, ALICE), true);
		assert_eq!(SharesAndWithdrawnRewards::<Runtime>::contains_key(1, ALICE), true);
		assert_eq!(SharesAndWithdrawnRewards::<Runtime>::contains_key(0, BOB), true);
		assert_eq!(SharesAndWithdrawnRewards::<Runtime>::contains_key(1, BOB), true);
		assert_eq!(SharesAndWithdrawnRewards::<Runtime>::contains_key(0, CAROL), true);

		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(0, ALICE),
			(100, vec![(NATIVE_COIN, 500), (STABLE_COIN, 0)].into_iter().collect()),
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(1, ALICE),
			(800, vec![(NATIVE_COIN, 200)].into_iter().collect()),
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(0, BOB),
			(500, vec![(NATIVE_COIN, 200)].into_iter().collect()),
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(1, BOB),
			(300, vec![(NATIVE_COIN, 200)].into_iter().collect()),
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(0, CAROL),
			(100, vec![(STABLE_COIN, 5000)].into_iter().collect()),
		);
	});
}
