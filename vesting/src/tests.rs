//! Unit tests for the vesting module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok, error::BadOrigin};
use mock::{Event, *};
use pallet_balances::{BalanceLock, Reasons};

#[test]
fn vesting_from_chain_spec_works() {
	ExtBuilder::build().execute_with(|| {
		assert_ok!(PalletBalances::ensure_can_withdraw(
			&CHARLIE,
			10,
			WithdrawReasons::TRANSFER,
			20
		));
		assert!(PalletBalances::ensure_can_withdraw(&CHARLIE, 11, WithdrawReasons::TRANSFER, 19).is_err());

		assert_eq!(
			Vesting::vesting_schedules(&CHARLIE),
			vec![VestingSchedule {
				start: 2u64,
				period: 3u64,
				period_count: 4u32,
				per_period: 5u64,
			}]
		);

		MockBlockNumberProvider::set(13);

		assert_ok!(Vesting::claim(Origin::signed(CHARLIE)));

		assert_ok!(PalletBalances::ensure_can_withdraw(
			&CHARLIE,
			25,
			WithdrawReasons::TRANSFER,
			5
		));
		assert!(PalletBalances::ensure_can_withdraw(&CHARLIE, 26, WithdrawReasons::TRANSFER, 4).is_err());

		MockBlockNumberProvider::set(14);

		assert_ok!(Vesting::claim(Origin::signed(CHARLIE)));

		assert_ok!(PalletBalances::ensure_can_withdraw(
			&CHARLIE,
			30,
			WithdrawReasons::TRANSFER,
			0
		));
	});
}

#[test]
fn vested_transfer_works() {
	ExtBuilder::build().execute_with(|| {
		System::set_block_number(1);

		let schedule = VestingSchedule {
			start: 0u64,
			period: 10u64,
			period_count: 1u32,
			per_period: 100u64,
		};
		assert_ok!(Vesting::vested_transfer(Origin::signed(ALICE), BOB, schedule.clone()));
		assert_eq!(Vesting::vesting_schedules(&BOB), vec![schedule.clone()]);
		System::assert_last_event(Event::Vesting(crate::Event::VestingScheduleAdded(ALICE, BOB, schedule)));
	});
}

#[test]
fn add_new_vesting_schedule_merges_with_current_locked_balance_and_until() {
	ExtBuilder::build().execute_with(|| {
		let schedule = VestingSchedule {
			start: 0u64,
			period: 10u64,
			period_count: 2u32,
			per_period: 10u64,
		};
		assert_ok!(Vesting::vested_transfer(Origin::signed(ALICE), BOB, schedule));

		MockBlockNumberProvider::set(12);

		let another_schedule = VestingSchedule {
			start: 10u64,
			period: 13u64,
			period_count: 1u32,
			per_period: 7u64,
		};
		assert_ok!(Vesting::vested_transfer(Origin::signed(ALICE), BOB, another_schedule));

		assert_eq!(
			PalletBalances::locks(&BOB).get(0),
			Some(&BalanceLock {
				id: VESTING_LOCK_ID,
				amount: 17u64,
				reasons: Reasons::All,
			})
		);
	});
}

#[test]
fn cannot_use_fund_if_not_claimed() {
	ExtBuilder::build().execute_with(|| {
		let schedule = VestingSchedule {
			start: 10u64,
			period: 10u64,
			period_count: 1u32,
			per_period: 50u64,
		};
		assert_ok!(Vesting::vested_transfer(Origin::signed(ALICE), BOB, schedule));
		assert!(PalletBalances::ensure_can_withdraw(&BOB, 1, WithdrawReasons::TRANSFER, 49).is_err());
	});
}

#[test]
fn vested_transfer_fails_if_zero_period_or_count() {
	ExtBuilder::build().execute_with(|| {
		let schedule = VestingSchedule {
			start: 1u64,
			period: 0u64,
			period_count: 1u32,
			per_period: 100u64,
		};
		assert_noop!(
			Vesting::vested_transfer(Origin::signed(ALICE), BOB, schedule),
			Error::<Runtime>::ZeroVestingPeriod
		);

		let schedule = VestingSchedule {
			start: 1u64,
			period: 1u64,
			period_count: 0u32,
			per_period: 100u64,
		};
		assert_noop!(
			Vesting::vested_transfer(Origin::signed(ALICE), BOB, schedule),
			Error::<Runtime>::ZeroVestingPeriodCount
		);
	});
}

#[test]
fn vested_transfer_fails_if_transfer_err() {
	ExtBuilder::build().execute_with(|| {
		let schedule = VestingSchedule {
			start: 1u64,
			period: 1u64,
			period_count: 1u32,
			per_period: 100u64,
		};
		assert_noop!(
			Vesting::vested_transfer(Origin::signed(BOB), ALICE, schedule),
			pallet_balances::Error::<Runtime, _>::InsufficientBalance,
		);
	});
}

#[test]
fn vested_transfer_fails_if_overflow() {
	ExtBuilder::build().execute_with(|| {
		let schedule = VestingSchedule {
			start: 1u64,
			period: 1u64,
			period_count: 2u32,
			per_period: u64::MAX,
		};
		assert_noop!(
			Vesting::vested_transfer(Origin::signed(ALICE), BOB, schedule),
			ArithmeticError::Overflow,
		);

		let another_schedule = VestingSchedule {
			start: u64::MAX,
			period: 1u64,
			period_count: 2u32,
			per_period: 1u64,
		};
		assert_noop!(
			Vesting::vested_transfer(Origin::signed(ALICE), BOB, another_schedule),
			ArithmeticError::Overflow,
		);
	});
}

#[test]
fn vested_transfer_fails_if_bad_origin() {
	ExtBuilder::build().execute_with(|| {
		let schedule = VestingSchedule {
			start: 0u64,
			period: 10u64,
			period_count: 1u32,
			per_period: 100u64,
		};
		assert_noop!(
			Vesting::vested_transfer(Origin::signed(CHARLIE), BOB, schedule),
			BadOrigin
		);
	});
}

#[test]
fn claim_works() {
	ExtBuilder::build().execute_with(|| {
		let schedule = VestingSchedule {
			start: 0u64,
			period: 10u64,
			period_count: 2u32,
			per_period: 10u64,
		};
		assert_ok!(Vesting::vested_transfer(Origin::signed(ALICE), BOB, schedule));

		MockBlockNumberProvider::set(11);
		// remain locked if not claimed
		assert!(PalletBalances::transfer(Origin::signed(BOB), ALICE, 10).is_err());
		// unlocked after claiming
		assert_ok!(Vesting::claim(Origin::signed(BOB)));
		assert!(VestingSchedules::<Runtime>::contains_key(BOB));
		assert_ok!(PalletBalances::transfer(Origin::signed(BOB), ALICE, 10));
		// more are still locked
		assert!(PalletBalances::transfer(Origin::signed(BOB), ALICE, 1).is_err());

		MockBlockNumberProvider::set(21);
		// claim more
		assert_ok!(Vesting::claim(Origin::signed(BOB)));
		assert!(!VestingSchedules::<Runtime>::contains_key(BOB));
		assert_ok!(PalletBalances::transfer(Origin::signed(BOB), ALICE, 10));
		// all used up
		assert_eq!(PalletBalances::free_balance(BOB), 0);

		// no locks anymore
		assert_eq!(PalletBalances::locks(&BOB), vec![]);
	});
}

#[test]
fn claim_for_works() {
	ExtBuilder::build().execute_with(|| {
		let schedule = VestingSchedule {
			start: 0u64,
			period: 10u64,
			period_count: 2u32,
			per_period: 10u64,
		};
		assert_ok!(Vesting::vested_transfer(Origin::signed(ALICE), BOB, schedule));

		assert_ok!(Vesting::claim_for(Origin::signed(ALICE), BOB));

		assert_eq!(
			PalletBalances::locks(&BOB).get(0),
			Some(&BalanceLock {
				id: VESTING_LOCK_ID,
				amount: 20u64,
				reasons: Reasons::All,
			})
		);
		assert!(VestingSchedules::<Runtime>::contains_key(&BOB));

		MockBlockNumberProvider::set(21);

		assert_ok!(Vesting::claim_for(Origin::signed(ALICE), BOB));

		// no locks anymore
		assert_eq!(PalletBalances::locks(&BOB), vec![]);
		assert!(!VestingSchedules::<Runtime>::contains_key(&BOB));
	});
}

#[test]
fn update_vesting_schedules_works() {
	ExtBuilder::build().execute_with(|| {
		let schedule = VestingSchedule {
			start: 0u64,
			period: 10u64,
			period_count: 2u32,
			per_period: 10u64,
		};
		assert_ok!(Vesting::vested_transfer(Origin::signed(ALICE), BOB, schedule));

		let updated_schedule = VestingSchedule {
			start: 0u64,
			period: 20u64,
			period_count: 2u32,
			per_period: 10u64,
		};
		assert_ok!(Vesting::update_vesting_schedules(
			Origin::root(),
			BOB,
			vec![updated_schedule]
		));

		MockBlockNumberProvider::set(11);
		assert_ok!(Vesting::claim(Origin::signed(BOB)));
		assert!(PalletBalances::transfer(Origin::signed(BOB), ALICE, 1).is_err());

		MockBlockNumberProvider::set(21);
		assert_ok!(Vesting::claim(Origin::signed(BOB)));
		assert_ok!(PalletBalances::transfer(Origin::signed(BOB), ALICE, 10));

		// empty vesting schedules cleanup the storage and unlock the fund
		assert!(VestingSchedules::<Runtime>::contains_key(BOB));
		assert_eq!(
			PalletBalances::locks(&BOB).get(0),
			Some(&BalanceLock {
				id: VESTING_LOCK_ID,
				amount: 10u64,
				reasons: Reasons::All,
			})
		);
		assert_ok!(Vesting::update_vesting_schedules(Origin::root(), BOB, vec![]));
		assert!(!VestingSchedules::<Runtime>::contains_key(BOB));
		assert_eq!(PalletBalances::locks(&BOB), vec![]);
	});
}

#[test]
fn update_vesting_schedules_fails_if_unexpected_existing_locks() {
	ExtBuilder::build().execute_with(|| {
		assert_ok!(PalletBalances::transfer(Origin::signed(ALICE), BOB, 1));
		PalletBalances::set_lock(*b"prelocks", &BOB, 0u64, WithdrawReasons::all());
	});
}

#[test]
fn vested_transfer_check_for_min() {
	ExtBuilder::build().execute_with(|| {
		let schedule = VestingSchedule {
			start: 1u64,
			period: 1u64,
			period_count: 1u32,
			per_period: 3u64,
		};
		assert_noop!(
			Vesting::vested_transfer(Origin::signed(BOB), ALICE, schedule),
			Error::<Runtime>::AmountLow
		);
	});
}

#[test]
fn multiple_vesting_schedule_claim_works() {
	ExtBuilder::build().execute_with(|| {
		let schedule = VestingSchedule {
			start: 0u64,
			period: 10u64,
			period_count: 2u32,
			per_period: 10u64,
		};
		assert_ok!(Vesting::vested_transfer(Origin::signed(ALICE), BOB, schedule.clone()));

		let schedule2 = VestingSchedule {
			start: 0u64,
			period: 10u64,
			period_count: 3u32,
			per_period: 10u64,
		};
		assert_ok!(Vesting::vested_transfer(Origin::signed(ALICE), BOB, schedule2.clone()));

		assert_eq!(Vesting::vesting_schedules(&BOB), vec![schedule, schedule2.clone()]);

		MockBlockNumberProvider::set(21);

		assert_ok!(Vesting::claim(Origin::signed(BOB)));

		assert_eq!(Vesting::vesting_schedules(&BOB), vec![schedule2]);

		MockBlockNumberProvider::set(31);

		assert_ok!(Vesting::claim(Origin::signed(BOB)));

		assert!(!VestingSchedules::<Runtime>::contains_key(&BOB));

		assert_eq!(PalletBalances::locks(&BOB), vec![]);
	});
}

#[test]
fn exceeding_maximum_schedules_should_fail() {
	ExtBuilder::build().execute_with(|| {
		let schedule = VestingSchedule {
			start: 0u64,
			period: 10u64,
			period_count: 2u32,
			per_period: 10u64,
		};
		assert_ok!(Vesting::vested_transfer(Origin::signed(ALICE), BOB, schedule.clone()));
		assert_ok!(Vesting::vested_transfer(Origin::signed(ALICE), BOB, schedule.clone()));
		assert_noop!(
			Vesting::vested_transfer(Origin::signed(ALICE), BOB, schedule.clone()),
			Error::<Runtime>::MaxVestingSchedulesExceeded
		);

		let schedules = vec![schedule.clone(), schedule.clone(), schedule];

		assert_noop!(
			Vesting::update_vesting_schedules(Origin::root(), BOB, schedules),
			Error::<Runtime>::MaxVestingSchedulesExceeded
		);
	});
}
