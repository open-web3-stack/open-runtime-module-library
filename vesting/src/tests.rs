//! Unit tests for the vesting module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok, traits::WithdrawReason};
use mock::{ExtBuilder, Origin, PalletBalances, Runtime, System, TestEvent, Vesting, ALICE, BOB};

#[test]
fn add_vesting_schedule_works() {
	ExtBuilder::default().one_hundred_for_alice().build().execute_with(|| {
		let schedule = VestingSchedule {
			start: 0u64,
			period: 10u64,
			period_count: 1u32,
			per_period: 100u64,
		};
		assert_ok!(Vesting::add_vesting_schedule(
			Origin::signed(ALICE),
			BOB,
			schedule.clone()
		));
		let vested = Vesting::vesting_schedules(&BOB);
		assert_eq!(vested, vec![schedule.clone()]);

		let vested_event = TestEvent::vesting(RawEvent::Vested(ALICE, BOB, schedule));
		assert!(System::events().iter().any(|record| record.event == vested_event));
	});
}

#[test]
fn cannot_use_fund_if_not_claimed() {
	ExtBuilder::default().one_hundred_for_alice().build().execute_with(|| {
		let schedule = VestingSchedule {
			start: 10u64,
			period: 10u64,
			period_count: 1u32,
			per_period: 50u64,
		};
		assert_ok!(Vesting::add_vesting_schedule(
			Origin::signed(ALICE),
			BOB,
			schedule.clone()
		));
		assert!(PalletBalances::ensure_can_withdraw(&BOB, 1, WithdrawReason::Transfer.into(), 49).is_err());
	});
}

#[test]
fn add_vesting_schedule_fails_if_zero_period_or_count() {
	ExtBuilder::default().one_hundred_for_alice().build().execute_with(|| {
		let schedule = VestingSchedule {
			start: 1u64,
			period: 0u64,
			period_count: 1u32,
			per_period: 100u64,
		};
		assert_noop!(
			Vesting::add_vesting_schedule(Origin::signed(ALICE), BOB, schedule.clone()),
			Error::<Runtime>::ZeroVestingPeriod
		);

		let schedule = VestingSchedule {
			start: 1u64,
			period: 1u64,
			period_count: 0u32,
			per_period: 100u64,
		};
		assert_noop!(
			Vesting::add_vesting_schedule(Origin::signed(ALICE), BOB, schedule.clone()),
			Error::<Runtime>::ZeroVestingPeriodCount
		);
	});
}

//TODO: to be implemented
#[test]
fn add_vesting_schedule_fails_if_unexpected_existing_locks() {
	ExtBuilder::default()
		.one_hundred_for_alice()
		.build()
		.execute_with(|| {});
}

#[test]
fn add_vesting_schedule_fails_if_transfer_err() {
	ExtBuilder::default().one_hundred_for_alice().build().execute_with(|| {
		let schedule = VestingSchedule {
			start: 1u64,
			period: 1u64,
			period_count: 1u32,
			per_period: 100u64,
		};
		assert_noop!(
			Vesting::add_vesting_schedule(Origin::signed(BOB), ALICE, schedule.clone()),
			pallet_balances::Error::<Runtime, _>::InsufficientBalance,
		);
	});
}
