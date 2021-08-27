#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{Event, *};

#[test]
fn should_feed_values_from_member() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let account_id: AccountId = 1;

		assert_noop!(
			ModuleOracle::feed_values(Origin::signed(5), vec![(50, 1000), (51, 900), (52, 800)]),
			Error::<Test, _>::NoPermission,
		);

		assert_eq!(
			ModuleOracle::feed_values(Origin::signed(account_id), vec![(50, 1000), (51, 900), (52, 800)])
				.unwrap()
				.pays_fee,
			Pays::No
		);
		System::assert_last_event(Event::ModuleOracle(crate::Event::NewFeedData(
			1,
			vec![(50, 1000), (51, 900), (52, 800)],
		)));

		assert_eq!(
			ModuleOracle::raw_values(&account_id, &50),
			Some(TimestampedValue {
				value: 1000,
				timestamp: 12345,
			})
		);

		assert_eq!(
			ModuleOracle::raw_values(&account_id, &51),
			Some(TimestampedValue {
				value: 900,
				timestamp: 12345,
			})
		);

		assert_eq!(
			ModuleOracle::raw_values(&account_id, &52),
			Some(TimestampedValue {
				value: 800,
				timestamp: 12345,
			})
		);
	});
}

#[test]
fn should_feed_values_from_root() {
	new_test_ext().execute_with(|| {
		let root_feeder: AccountId = RootOperatorAccountId::get();

		assert_ok!(ModuleOracle::feed_values(
			Origin::root(),
			vec![(50, 1000), (51, 900), (52, 800)]
		));

		assert_eq!(
			ModuleOracle::raw_values(&root_feeder, &50),
			Some(TimestampedValue {
				value: 1000,
				timestamp: 12345,
			})
		);

		assert_eq!(
			ModuleOracle::raw_values(&root_feeder, &51),
			Some(TimestampedValue {
				value: 900,
				timestamp: 12345,
			})
		);

		assert_eq!(
			ModuleOracle::raw_values(&root_feeder, &52),
			Some(TimestampedValue {
				value: 800,
				timestamp: 12345,
			})
		);
	});
}

#[test]
fn should_update_is_updated() {
	new_test_ext().execute_with(|| {
		let key: u32 = 50;
		assert!(!ModuleOracle::is_updated(key));
		assert_ok!(ModuleOracle::feed_values(Origin::signed(1), vec![(key, 1000)]));
		assert_ok!(ModuleOracle::feed_values(Origin::signed(2), vec![(key, 1000)]));
		assert_ok!(ModuleOracle::feed_values(Origin::signed(3), vec![(key, 1000)]));
		assert!(!ModuleOracle::is_updated(key));
		assert_eq!(
			ModuleOracle::get(&key).unwrap(),
			TimestampedValue {
				value: 1000,
				timestamp: 12345
			}
		);
		assert!(ModuleOracle::is_updated(key));
		ModuleOracle::on_finalize(1);
		assert_ok!(ModuleOracle::feed_values(Origin::signed(1), vec![(key, 1000)]));
		assert!(!ModuleOracle::is_updated(key));
	});
}

#[test]
fn should_read_raw_values() {
	new_test_ext().execute_with(|| {
		let key: u32 = 50;

		let raw_values = ModuleOracle::read_raw_values(&key);
		assert_eq!(raw_values, vec![]);

		assert_ok!(ModuleOracle::feed_values(Origin::signed(1), vec![(key, 1000)]));
		assert_ok!(ModuleOracle::feed_values(Origin::signed(2), vec![(key, 1200)]));

		let raw_values = ModuleOracle::read_raw_values(&key);
		assert_eq!(
			raw_values,
			vec![
				TimestampedValue {
					value: 1000,
					timestamp: 12345,
				},
				TimestampedValue {
					value: 1200,
					timestamp: 12345,
				},
			]
		);
	});
}

#[test]
fn should_combined_data() {
	new_test_ext().execute_with(|| {
		let key: u32 = 50;

		assert_ok!(ModuleOracle::feed_values(Origin::signed(1), vec![(key, 1300)]));
		assert_ok!(ModuleOracle::feed_values(Origin::signed(2), vec![(key, 1000)]));
		assert_ok!(ModuleOracle::feed_values(Origin::signed(3), vec![(key, 1200)]));

		let expected = Some(TimestampedValue {
			value: 1200,
			timestamp: 12345,
		});

		assert_eq!(ModuleOracle::get(&key), expected);

		Timestamp::set_timestamp(23456);

		assert_eq!(ModuleOracle::get(&key), expected);
	});
}

#[test]
fn should_return_none_for_non_exist_key() {
	new_test_ext().execute_with(|| {
		assert_eq!(ModuleOracle::get(&50), None);
	});
}

#[test]
fn multiple_calls_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleOracle::feed_values(Origin::signed(1), vec![(50, 1300)]));
		assert_noop!(
			ModuleOracle::feed_values(Origin::signed(1), vec![(50, 1300)]),
			Error::<Test, _>::AlreadyFeeded,
		);

		ModuleOracle::on_finalize(1);

		assert_ok!(ModuleOracle::feed_values(Origin::signed(1), vec![(50, 1300)]));
	});
}

#[test]
fn get_all_values_should_work() {
	new_test_ext().execute_with(|| {
		let eur: u32 = 1;
		let jpy: u32 = 2;

		assert_eq!(ModuleOracle::get_all_values(), vec![]);

		// feed eur & jpy
		assert_ok!(ModuleOracle::feed_values(Origin::signed(1), vec![(eur, 1300)]));
		assert_ok!(ModuleOracle::feed_values(Origin::signed(2), vec![(eur, 1000)]));
		assert_ok!(ModuleOracle::feed_values(Origin::signed(3), vec![(jpy, 9000)]));

		// not enough eur & jpy prices
		assert_eq!(ModuleOracle::get(&eur), None);
		assert_eq!(ModuleOracle::get(&jpy), None);
		assert_eq!(ModuleOracle::get_all_values(), vec![]);

		// finalize block
		ModuleOracle::on_finalize(1);

		// feed eur & jpy
		assert_ok!(ModuleOracle::feed_values(Origin::signed(3), vec![(eur, 1200)]));
		assert_ok!(ModuleOracle::feed_values(Origin::signed(1), vec![(jpy, 8000)]));

		// enough eur prices
		let eur_price = Some(TimestampedValue {
			value: 1200,
			timestamp: 12345,
		});
		assert_eq!(ModuleOracle::get(&eur), eur_price);

		// not enough jpy prices
		assert_eq!(ModuleOracle::get(&jpy), None);

		assert_eq!(ModuleOracle::get_all_values(), vec![(eur, eur_price)]);

		// feed jpy
		assert_ok!(ModuleOracle::feed_values(Origin::signed(2), vec![(jpy, 7000)]));

		// enough jpy prices
		let jpy_price = Some(TimestampedValue {
			value: 8000,
			timestamp: 12345,
		});
		assert_eq!(ModuleOracle::get(&jpy), jpy_price);

		assert_eq!(ModuleOracle::get_all_values(), vec![(eur, eur_price), (jpy, jpy_price)]);
	});
}

#[test]
fn change_member_should_work() {
	new_test_ext().execute_with(|| {
		OracleMembers::set(vec![2, 3, 4]);
		<ModuleOracle as ChangeMembers<AccountId>>::change_members_sorted(&[4], &[1], &[2, 3, 4]);
		assert_noop!(
			ModuleOracle::feed_values(Origin::signed(1), vec![(50, 1000)]),
			Error::<Test, _>::NoPermission,
		);
		assert_ok!(ModuleOracle::feed_values(Origin::signed(2), vec![(50, 1000)]));
		assert_ok!(ModuleOracle::feed_values(Origin::signed(4), vec![(50, 1000)]));
	});
}

#[test]
fn should_clear_is_updated_on_change_member() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleOracle::feed_values(Origin::signed(1), vec![(50, 1000)]));
		assert_ok!(ModuleOracle::feed_values(Origin::signed(2), vec![(50, 1000)]));
		assert_ok!(ModuleOracle::feed_values(Origin::signed(3), vec![(50, 1000)]));

		assert_eq!(
			ModuleOracle::get(&50).unwrap(),
			TimestampedValue {
				value: 1000,
				timestamp: 12345
			}
		);
		assert!(ModuleOracle::is_updated(50));

		ModuleOracle::change_members_sorted(&[4], &[1], &[2, 3, 4]);

		assert!(!ModuleOracle::is_updated(50));
	});
}

#[test]
fn should_clear_data_for_removed_members() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleOracle::feed_values(Origin::signed(1), vec![(50, 1000)]));
		assert_ok!(ModuleOracle::feed_values(Origin::signed(2), vec![(50, 1000)]));

		ModuleOracle::change_members_sorted(&[4], &[1], &[2, 3, 4]);

		assert_eq!(ModuleOracle::raw_values(&1, 50), None);
	});
}
