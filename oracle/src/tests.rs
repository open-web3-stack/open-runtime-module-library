#![cfg(test)]

use crate::mock::{new_test_ext, MockTime, ModuleOracle, Origin};

use crate::TimestampedValue;
use support::assert_ok;

#[test]
fn should_feed_data() {
	new_test_ext().execute_with(|| {
		let key: u32 = 1;
		let account_id: u64 = 1;

		MockTime::set_time(12345);

		let expected = TimestampedValue {
			value: 1000,
			timestamp: 12345,
		};

		assert_ok!(ModuleOracle::feed_data(Origin::signed(account_id), key, 1000));

		let feed_data = ModuleOracle::raw_values((account_id, key)).unwrap();
		assert_eq!(feed_data, expected);
	});
}

#[test]
fn should_change_status_when_feeding() {
	new_test_ext().execute_with(|| {
		let key: u32 = 1;
		assert_eq!(ModuleOracle::has_update(key), false);
		assert_ok!(ModuleOracle::feed_data(Origin::signed(1), key, 1000));
		assert_eq!(ModuleOracle::has_update(key), true);
	});
}

#[test]
fn should_read_raw_values() {
	new_test_ext().execute_with(|| {
		let key: u32 = 1;

		let raw_values = ModuleOracle::read_raw_values(&key);
		assert_eq!(raw_values, vec![]);

		MockTime::set_time(12345);

		let expected = vec![
			TimestampedValue {
				value: 1000,
				timestamp: 12345,
			},
			TimestampedValue {
				value: 1200,
				timestamp: 12345,
			},
		];

		assert_ok!(ModuleOracle::feed_data(Origin::signed(1), key, 1000));
		assert_ok!(ModuleOracle::feed_data(Origin::signed(2), key, 1200));

		let raw_values = ModuleOracle::read_raw_values(&key);
		assert_eq!(raw_values, expected);
	});
}

#[test]
fn should_get_combined_data() {
	new_test_ext().execute_with(|| {
		MockTime::set_time(12345);
		let expected = Some(TimestampedValue {
			value: 1000,
			timestamp: 12345,
		});

		let key: u32 = 1;

		assert_ok!(ModuleOracle::feed_data(Origin::signed(1), key, 1000));
		assert_ok!(ModuleOracle::feed_data(Origin::signed(2), key, 1200));
		assert_ok!(ModuleOracle::feed_data(Origin::signed(3), key, 1300));
		assert_eq!(ModuleOracle::get(&key), expected);
	});
}
