#![cfg(test)]

use crate::mock::{new_test_ext, MockTime, ModuleOracle, Origin};

use crate::TimestampedValue;
use support::assert_ok;

#[test]
fn should_feed_data() {
	new_test_ext().execute_with(|| {
		MockTime::set_time(12345);

		let expected = TimestampedValue {
			value: 1000,
			timestamp: 12345,
		};

		assert_ok!(ModuleOracle::feed_data(Origin::signed(1), 1, 1000));

		let feed_data = ModuleOracle::raw_values((1, 1)).unwrap();
		assert_eq!(feed_data, expected);
	});
}

#[test]
fn should_change_status_when_feeding() {
	new_test_ext().execute_with(|| {
		assert_eq!(ModuleOracle::has_update(1), false);
		assert_ok!(ModuleOracle::feed_data(Origin::signed(1), 1, 1000));
		assert_eq!(ModuleOracle::has_update(1), true);
	});
}

#[test]
fn should_read_raw_values() {
	new_test_ext().execute_with(|| {
		let raw_values = ModuleOracle::read_raw_values(&1);
		assert_eq!(raw_values, vec![]);

		MockTime::set_time(12345);

		let expected = [
			TimestampedValue {
				value: 1000,
				timestamp: 12345,
			},
			TimestampedValue {
				value: 1200,
				timestamp: 12345,
			},
		];

		assert_ok!(ModuleOracle::feed_data(Origin::signed(1), 1, 1000));
		assert_ok!(ModuleOracle::feed_data(Origin::signed(2), 1, 1200));

		let raw_values = ModuleOracle::read_raw_values(&1);
		assert_eq!(raw_values, expected);
	});
}
