//! Unit tests for the rewards module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;

#[test]
fn add_share_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(RewardsModule::pool_infos(DOT_POOL), Default::default());
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, ALICE),
			Default::default()
		);

		RewardsModule::add_share(&ALICE, &DOT_POOL, 0);
		assert_eq!(RewardsModule::pool_infos(DOT_POOL), Default::default());
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, ALICE),
			Default::default()
		);

		RewardsModule::add_share(&ALICE, &DOT_POOL, 100);

		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 100,
				..Default::default()
			}
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(&DOT_POOL, &ALICE),
			(100, Default::default())
		);

		PoolInfos::<Runtime>::mutate(DOT_POOL, |pool_info| {
			pool_info.rewards.insert(NATIVE_COIN, (5_000, 2_000));
		});

		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 100,
				rewards: vec![(NATIVE_COIN, (5_000, 2_000))].into_iter().collect()
			}
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, BOB),
			Default::default()
		);

		RewardsModule::add_share(&BOB, &DOT_POOL, 50);

		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 150,
				rewards: vec![(NATIVE_COIN, (7_500, 4_500))].into_iter().collect()
			}
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, BOB),
			(50, vec![(NATIVE_COIN, 2_500)].into_iter().collect())
		);

		RewardsModule::add_share(&ALICE, &DOT_POOL, 100);

		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 250,
				rewards: vec![(NATIVE_COIN, (12_500, 9_500))].into_iter().collect()
			}
		);

		RewardsModule::add_share(&ALICE, &DOT_POOL, 50);

		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 300,
				rewards: vec![(NATIVE_COIN, (15_000, 12_000))].into_iter().collect()
			}
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, ALICE),
			(250, vec![(NATIVE_COIN, 7_500)].into_iter().collect())
		);

		// overflow occurs when saturating calculation
		RewardsModule::add_share(&ALICE, &DOT_POOL, u64::MAX);

		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: u64::MAX,
				rewards: vec![(NATIVE_COIN, (u64::MAX, u64::MAX))].into_iter().collect()
			}
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, ALICE),
			(u64::MAX, vec![(NATIVE_COIN, u64::MAX)].into_iter().collect())
		);
	});
}

#[test]
fn claim_rewards_should_not_create_empty_records() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(PoolInfos::<Runtime>::contains_key(&DOT_POOL), false);
		assert_eq!(
			SharesAndWithdrawnRewards::<Runtime>::contains_key(&DOT_POOL, &ALICE),
			false
		);
		RewardsModule::claim_rewards(&ALICE, &DOT_POOL);
		assert_eq!(PoolInfos::<Runtime>::contains_key(&DOT_POOL), false);
		assert_eq!(
			SharesAndWithdrawnRewards::<Runtime>::contains_key(&DOT_POOL, &ALICE),
			false
		);
	})
}

#[test]
fn claim_rewards_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		RewardsModule::add_share(&ALICE, &DOT_POOL, 100);
		RewardsModule::add_share(&BOB, &DOT_POOL, 100);
		PoolInfos::<Runtime>::mutate(DOT_POOL, |pool_info| {
			pool_info.rewards.insert(NATIVE_COIN, (5_000, 0));
		});
		RewardsModule::add_share(&CAROL, &DOT_POOL, 200);
		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 400,
				rewards: vec![(NATIVE_COIN, (10_000, 5_000))].into_iter().collect()
			}
		);

		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, ALICE),
			(100, Default::default())
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, BOB),
			(100, Default::default())
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, CAROL),
			(200, vec![(NATIVE_COIN, 5_000)].into_iter().collect())
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, ALICE, NATIVE_COIN)).unwrap_or(&0)),
			0
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, BOB, NATIVE_COIN)).unwrap_or(&0)),
			0
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, CAROL, NATIVE_COIN)).unwrap_or(&0)),
			0
		);

		RewardsModule::claim_rewards(&ALICE, &DOT_POOL);
		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 400,
				rewards: vec![(NATIVE_COIN, (10_000, 7_500))].into_iter().collect()
			}
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, ALICE),
			(100, vec![(NATIVE_COIN, 2_500)].into_iter().collect())
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, ALICE, NATIVE_COIN)).unwrap_or(&0)),
			2_500
		);

		RewardsModule::claim_rewards(&CAROL, &DOT_POOL);
		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 400,
				rewards: vec![(NATIVE_COIN, (10_000, 7_500))].into_iter().collect()
			}
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, CAROL),
			(200, vec![(NATIVE_COIN, 5_000)].into_iter().collect())
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, CAROL, NATIVE_COIN)).unwrap_or(&0)),
			0
		);

		RewardsModule::claim_rewards(&BOB, &DOT_POOL);
		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 400,
				rewards: vec![(NATIVE_COIN, (10_000, 10_000))].into_iter().collect()
			}
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, BOB),
			(100, vec![(NATIVE_COIN, 2_500)].into_iter().collect())
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, BOB, NATIVE_COIN)).unwrap_or(&0)),
			2_500
		);
	});
}

#[test]
fn remove_share_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		RewardsModule::add_share(&ALICE, &DOT_POOL, 100);
		RewardsModule::add_share(&BOB, &DOT_POOL, 100);
		PoolInfos::<Runtime>::mutate(DOT_POOL, |pool_info| {
			pool_info.rewards.insert(NATIVE_COIN, (10_000, 0));
		});

		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 200,
				rewards: vec![(NATIVE_COIN, (10_000, 0))].into_iter().collect()
			}
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, ALICE),
			(100, Default::default())
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, BOB),
			(100, Default::default())
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, ALICE, NATIVE_COIN)).unwrap_or(&0)),
			0
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, BOB, NATIVE_COIN)).unwrap_or(&0)),
			0
		);

		// remove amount is zero, do not claim interest
		RewardsModule::remove_share(&ALICE, &DOT_POOL, 0);
		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 200,
				rewards: vec![(NATIVE_COIN, (10_000, 0))].into_iter().collect()
			}
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, ALICE),
			(100, Default::default())
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, ALICE, NATIVE_COIN)).unwrap_or(&0)),
			0
		);

		RewardsModule::remove_share(&BOB, &DOT_POOL, 50);
		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 150,
				rewards: vec![(NATIVE_COIN, (7_500, 2_500))].into_iter().collect()
			}
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, BOB),
			(50, vec![(NATIVE_COIN, 2_500)].into_iter().collect())
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, BOB, NATIVE_COIN)).unwrap_or(&0)),
			5_000
		);

		RewardsModule::remove_share(&ALICE, &DOT_POOL, 101);
		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 50,
				rewards: vec![(NATIVE_COIN, (2_500, 2_500))].into_iter().collect()
			}
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, ALICE),
			(0, Default::default())
		);
		assert_eq!(
			SharesAndWithdrawnRewards::<Runtime>::contains_key(&DOT_POOL, &ALICE),
			false
		);

		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, ALICE, NATIVE_COIN)).unwrap_or(&0)),
			5_000
		);

		// remove all shares will remove entries
		RewardsModule::remove_share(&BOB, &DOT_POOL, 100);
		assert_eq!(RewardsModule::pool_infos(DOT_POOL), PoolInfo::default());
		assert_eq!(PoolInfos::<Runtime>::contains_key(DOT_POOL), false);
		assert_eq!(PoolInfos::<Runtime>::iter().count(), 0);
		assert_eq!(
			SharesAndWithdrawnRewards::<Runtime>::contains_key(&DOT_POOL, &BOB),
			false
		);
		assert_eq!(SharesAndWithdrawnRewards::<Runtime>::iter().count(), 0);
	});
}

#[test]
fn set_share_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(RewardsModule::pool_infos(DOT_POOL), Default::default());
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, ALICE),
			Default::default()
		);

		RewardsModule::set_share(&ALICE, &DOT_POOL, 100);

		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 100,
				..Default::default()
			}
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, ALICE),
			(100, Default::default())
		);

		PoolInfos::<Runtime>::mutate(DOT_POOL, |pool_info| {
			pool_info.rewards.insert(NATIVE_COIN, (10_000, 0));
		});
		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 100,
				rewards: vec![(NATIVE_COIN, (10_000, 0))].into_iter().collect()
			}
		);

		RewardsModule::set_share(&ALICE, &DOT_POOL, 500);
		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 500,
				rewards: vec![(NATIVE_COIN, (50_000, 40_000))].into_iter().collect()
			}
		);

		PoolInfos::<Runtime>::mutate(DOT_POOL, |pool_info| {
			pool_info.rewards.insert(STABLE_COIN, (5_000, 0));
		});
		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 500,
				rewards: vec![(NATIVE_COIN, (50_000, 40_000)), (STABLE_COIN, (5_000, 0))]
					.into_iter()
					.collect()
			}
		);

		RewardsModule::set_share(&ALICE, &DOT_POOL, 600);
		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 600,
				rewards: vec![(NATIVE_COIN, (60_000, 50_000)), (STABLE_COIN, (6_000, 1_000))]
					.into_iter()
					.collect()
			}
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, ALICE),
			(
				600,
				vec![(NATIVE_COIN, 50_000), (STABLE_COIN, 1_000)].into_iter().collect()
			)
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, ALICE, NATIVE_COIN)).unwrap_or(&0)),
			0
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, ALICE, STABLE_COIN)).unwrap_or(&0)),
			0
		);

		RewardsModule::set_share(&ALICE, &DOT_POOL, 100);
		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 100,
				rewards: vec![(NATIVE_COIN, (10_000, 10_000)), (STABLE_COIN, (1_000, 1_000))]
					.into_iter()
					.collect()
			}
		);
		assert_eq!(
			RewardsModule::shares_and_withdrawn_rewards(DOT_POOL, ALICE),
			(
				100,
				vec![(NATIVE_COIN, 10_000), (STABLE_COIN, 1_000)].into_iter().collect()
			)
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, ALICE, NATIVE_COIN)).unwrap_or(&0)),
			10_000
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, ALICE, STABLE_COIN)).unwrap_or(&0)),
			5_000
		);
	});
}

#[test]
fn accumulate_reward_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(RewardsModule::pool_infos(DOT_POOL), Default::default());

		// should not accumulate if pool doesn't exist
		assert_noop!(
			RewardsModule::accumulate_reward(&DOT_POOL, NATIVE_COIN, 100),
			Error::<Runtime>::PoolDoesNotExist
		);
		assert_eq!(RewardsModule::pool_infos(DOT_POOL), PoolInfo::default());

		RewardsModule::add_share(&ALICE, &DOT_POOL, 100);

		assert_ok!(RewardsModule::accumulate_reward(&DOT_POOL, NATIVE_COIN, 100));
		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 100,
				rewards: vec![(NATIVE_COIN, (100, 0))].into_iter().collect(),
			}
		);

		assert_ok!(RewardsModule::accumulate_reward(&DOT_POOL, STABLE_COIN, 200));
		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 100,
				rewards: vec![(NATIVE_COIN, (100, 0)), (STABLE_COIN, (200, 0))]
					.into_iter()
					.collect(),
			}
		);
	});
}

#[test]
fn share_to_zero_removes_storage() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(
			SharesAndWithdrawnRewards::<Runtime>::contains_key(DOT_POOL, ALICE),
			false
		);
		RewardsModule::add_share(&ALICE, &DOT_POOL, 100);
		RewardsModule::add_share(&BOB, &DOT_POOL, 100);
		PoolInfos::<Runtime>::mutate(DOT_POOL, |pool_info| {
			pool_info.rewards.insert(NATIVE_COIN, (10000, 0));
		});

		assert_eq!(
			RewardsModule::pool_infos(DOT_POOL),
			PoolInfo {
				total_shares: 200,
				rewards: vec![(NATIVE_COIN, (10_000, 0))].into_iter().collect()
			}
		);

		// checks if key is removed
		assert_eq!(
			SharesAndWithdrawnRewards::<Runtime>::contains_key(DOT_POOL, ALICE),
			true
		);
		RewardsModule::remove_share(&ALICE, &DOT_POOL, 100);
		assert_eq!(
			SharesAndWithdrawnRewards::<Runtime>::contains_key(DOT_POOL, ALICE),
			false
		);

		RewardsModule::remove_share(&BOB, &DOT_POOL, 50);
		assert_eq!(SharesAndWithdrawnRewards::<Runtime>::contains_key(DOT_POOL, BOB), true);

		RewardsModule::remove_share(&BOB, &DOT_POOL, 100);
		assert_eq!(SharesAndWithdrawnRewards::<Runtime>::contains_key(DOT_POOL, BOB), false);
	});
}
