//! Unit tests for the rewards module.

#![cfg(test)]

use super::*;
use mock::*;

#[test]
fn add_share_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let rewards = BTreeMap::<CurrencyId, (Balance, Balance)>::new();
		let mut pool_info = PoolInfo {
			total_shares: 0,
			rewards,
		};
		let mut alice_withdrawn = BTreeMap::<CurrencyId, Balance>::new();
		let mut bob_withdrawn = BTreeMap::<CurrencyId, Balance>::new();

		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, ALICE),
			(0, alice_withdrawn.clone())
		);

		RewardsModule::add_share(&ALICE, &DOT_POOL, 0);
		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, ALICE),
			(0, alice_withdrawn.clone())
		);

		RewardsModule::add_share(&ALICE, &DOT_POOL, 100);
		pool_info.total_shares += 100;
		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(&DOT_POOL, &ALICE),
			(100, alice_withdrawn.clone())
		);

		Pools::<Runtime>::mutate(DOT_POOL, |pool_info| {
			pool_info.rewards.insert(NATIVE_COIN, (5000, 2000));
		});

		pool_info.rewards.insert(NATIVE_COIN, (5000, 2000));
		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, BOB),
			(0, bob_withdrawn.clone())
		);

		RewardsModule::add_share(&BOB, &DOT_POOL, 50);

		pool_info.total_shares = 150;
		pool_info.rewards.insert(NATIVE_COIN, (7500, 4500));
		bob_withdrawn.insert(NATIVE_COIN, 2500);

		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, BOB),
			(50, bob_withdrawn.clone())
		);

		RewardsModule::add_share(&ALICE, &DOT_POOL, 150);
		pool_info.total_shares = 300;
		pool_info.rewards.insert(NATIVE_COIN, (15000, 12000));
		alice_withdrawn.insert(NATIVE_COIN, 7500);
		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, ALICE),
			(250, alice_withdrawn.clone())
		);

		// overflow occurs when saturating calculation
		RewardsModule::add_share(&ALICE, &DOT_POOL, u64::MAX);
		pool_info.total_shares = u64::MAX;
		pool_info.rewards.insert(NATIVE_COIN, (u64::MAX, u64::MAX));
		alice_withdrawn.insert(NATIVE_COIN, u64::MAX);
		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, ALICE),
			(u64::MAX, alice_withdrawn.clone())
		);
	});
}

#[test]
fn claim_rewards_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let rewards = BTreeMap::<CurrencyId, (Balance, Balance)>::new();
		let mut pool_info = PoolInfo {
			total_shares: 0,
			rewards,
		};
		let mut alice_withdrawn = BTreeMap::<CurrencyId, Balance>::new();
		let mut bob_withdrawn = BTreeMap::<CurrencyId, Balance>::new();
		let mut carol_withdrawn = BTreeMap::<CurrencyId, Balance>::new();

		RewardsModule::add_share(&ALICE, &DOT_POOL, 100);
		RewardsModule::add_share(&BOB, &DOT_POOL, 100);
		Pools::<Runtime>::mutate(DOT_POOL, |pool_info| {
			pool_info.rewards.insert(NATIVE_COIN, (5000, 0));
		});
		RewardsModule::add_share(&CAROL, &DOT_POOL, 200);

		pool_info.total_shares = 400;
		pool_info.rewards.insert(NATIVE_COIN, (10000, 5000));
		carol_withdrawn.insert(NATIVE_COIN, 5000);
		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());

		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, ALICE),
			(100, alice_withdrawn.clone())
		);
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, BOB),
			(100, bob_withdrawn.clone())
		);
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, CAROL),
			(200, carol_withdrawn.clone())
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
		pool_info.rewards.insert(NATIVE_COIN, (10000, 7500));
		alice_withdrawn.insert(NATIVE_COIN, 2500);
		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, ALICE),
			(100, alice_withdrawn.clone())
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, ALICE, NATIVE_COIN)).unwrap_or(&0)),
			2500
		);

		RewardsModule::claim_rewards(&CAROL, &DOT_POOL);
		pool_info.rewards.insert(NATIVE_COIN, (10000, 7500));
		carol_withdrawn.insert(NATIVE_COIN, 5000);
		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, CAROL),
			(200, carol_withdrawn)
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, CAROL, NATIVE_COIN)).unwrap_or(&0)),
			0
		);

		RewardsModule::claim_rewards(&BOB, &DOT_POOL);
		pool_info.rewards.insert(NATIVE_COIN, (10000, 10000));
		bob_withdrawn.insert(NATIVE_COIN, 2500);
		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, BOB),
			(100, bob_withdrawn.clone())
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, BOB, NATIVE_COIN)).unwrap_or(&0)),
			2500
		);
	});
}

#[test]
fn remove_share_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let rewards = BTreeMap::<CurrencyId, (Balance, Balance)>::new();
		let mut pool_info = PoolInfo {
			total_shares: 200,
			rewards,
		};
		let alice_withdrawn = BTreeMap::<CurrencyId, Balance>::new();
		let mut bob_withdrawn = BTreeMap::<CurrencyId, Balance>::new();

		RewardsModule::add_share(&ALICE, &DOT_POOL, 100);
		RewardsModule::add_share(&BOB, &DOT_POOL, 100);
		Pools::<Runtime>::mutate(DOT_POOL, |pool_info| {
			pool_info.rewards.insert(NATIVE_COIN, (10000, 0));
		});

		pool_info.rewards.insert(NATIVE_COIN, (10000, 0));
		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, ALICE),
			(100, alice_withdrawn.clone())
		);
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, BOB),
			(100, bob_withdrawn.clone())
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
		pool_info.total_shares = 200;
		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, ALICE),
			(100, alice_withdrawn.clone())
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, ALICE, NATIVE_COIN)).unwrap_or(&0)),
			0
		);

		RewardsModule::remove_share(&BOB, &DOT_POOL, 50);
		pool_info.total_shares = 150;
		pool_info.rewards.insert(NATIVE_COIN, (7500, 2500));
		bob_withdrawn.insert(NATIVE_COIN, 2500);
		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, BOB),
			(50, bob_withdrawn.clone())
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, BOB, NATIVE_COIN)).unwrap_or(&0)),
			5000
		);

		RewardsModule::remove_share(&ALICE, &DOT_POOL, 101);
		pool_info.total_shares = 50;
		pool_info.rewards.insert(NATIVE_COIN, (2501, 2500));
		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, ALICE),
			(0, Default::default())
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, ALICE, NATIVE_COIN)).unwrap_or(&0)),
			4999
		);
	});
}

#[test]
fn set_share_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let rewards = BTreeMap::<CurrencyId, (Balance, Balance)>::new();
		let mut pool_info = PoolInfo {
			total_shares: 0,
			rewards,
		};
		let mut alice_withdrawn = BTreeMap::<CurrencyId, Balance>::new();
		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, ALICE),
			(0, alice_withdrawn.clone())
		);

		RewardsModule::set_share(&ALICE, &DOT_POOL, 100);
		pool_info.total_shares = 100;
		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, ALICE),
			(100, alice_withdrawn.clone())
		);

		Pools::<Runtime>::mutate(DOT_POOL, |pool_info| {
			pool_info.rewards.insert(NATIVE_COIN, (10000, 0));
		});
		pool_info.rewards.insert(NATIVE_COIN, (10000, 0));
		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());

		RewardsModule::set_share(&ALICE, &DOT_POOL, 500);
		pool_info.total_shares = 500;
		pool_info.rewards.insert(NATIVE_COIN, (50000, 40000));
		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());
		alice_withdrawn.insert(NATIVE_COIN, 40000);
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, ALICE),
			(500, alice_withdrawn.clone())
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, ALICE, NATIVE_COIN)).unwrap_or(&0)),
			0
		);

		RewardsModule::set_share(&ALICE, &DOT_POOL, 100);
		pool_info.total_shares = 100;
		pool_info.rewards.insert(NATIVE_COIN, (10000, 10000));
		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());
		alice_withdrawn.insert(NATIVE_COIN, 10000);
		assert_eq!(
			RewardsModule::share_and_withdrawn_reward(DOT_POOL, ALICE),
			(100, alice_withdrawn.clone())
		);
		assert_eq!(
			RECEIVED_PAYOUT.with(|v| *v.borrow().get(&(DOT_POOL, ALICE, NATIVE_COIN)).unwrap_or(&0)),
			10000
		);
	});
}

#[test]
fn accumulate_reward_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let rewards = BTreeMap::<CurrencyId, (Balance, Balance)>::new();
		let mut pool_info = PoolInfo {
			total_shares: 0,
			rewards,
		};
		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());

		RewardsModule::accumulate_reward(&DOT_POOL, NATIVE_COIN, 100);
		pool_info.rewards.insert(NATIVE_COIN, (100, 0));
		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());
	});
}

#[test]
fn share_to_zero_removes_storage() {
	ExtBuilder::default().build().execute_with(|| {
		let rewards = BTreeMap::<CurrencyId, (Balance, Balance)>::new();
		let mut pool_info = PoolInfo {
			total_shares: 200,
			rewards,
		};
		pool_info.rewards.insert(NATIVE_COIN, (10000, 0));
		assert_eq!(ShareAndWithdrawnReward::<Runtime>::contains_key(DOT_POOL, ALICE), false);
		RewardsModule::add_share(&ALICE, &DOT_POOL, 100);
		RewardsModule::add_share(&BOB, &DOT_POOL, 100);
		Pools::<Runtime>::mutate(DOT_POOL, |pool_info| {
			pool_info.rewards.insert(NATIVE_COIN, (10000, 0));
		});

		assert_eq!(RewardsModule::pools(DOT_POOL), pool_info.clone());

		// checks if key is removed
		assert_eq!(ShareAndWithdrawnReward::<Runtime>::contains_key(DOT_POOL, ALICE), true);
		RewardsModule::remove_share(&ALICE, &DOT_POOL, 100);
		assert_eq!(ShareAndWithdrawnReward::<Runtime>::contains_key(DOT_POOL, ALICE), false);

		RewardsModule::remove_share(&BOB, &DOT_POOL, 50);
		assert_eq!(ShareAndWithdrawnReward::<Runtime>::contains_key(DOT_POOL, BOB), true);

		RewardsModule::remove_share(&BOB, &DOT_POOL, 100);
		assert_eq!(ShareAndWithdrawnReward::<Runtime>::contains_key(DOT_POOL, BOB), false);
	});
}
