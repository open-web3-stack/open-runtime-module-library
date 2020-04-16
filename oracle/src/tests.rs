#![cfg(test)]

use crate::{
	mock::{new_test_ext, Call, ModuleOracle, OracleCall, Origin, Test, Timestamp},
	{CheckOperator, Error, TimestampedValue},
};
use frame_support::{
	assert_noop, assert_ok,
	traits::OnFinalize,
	weights::{DispatchClass, DispatchInfo, GetDispatchInfo, TransactionPriority},
};
use sp_runtime::{traits::SignedExtension, transaction_validity::ValidTransaction};

#[test]
fn should_feed_value() {
	new_test_ext().execute_with(|| {
		let key: u32 = 1;
		let account_id: u64 = 1;

		Timestamp::set_timestamp(12345);

		let expected = TimestampedValue {
			value: 1000,
			timestamp: 12345,
		};

		assert_ok!(ModuleOracle::feed_value(Origin::signed(account_id), key, 1000));

		let feed_data = ModuleOracle::raw_values(&key, &account_id).unwrap();
		assert_eq!(feed_data, expected);
	});
}

#[test]
fn should_feed_values() {
	new_test_ext().execute_with(|| {
		let account_id: u64 = 1;

		Timestamp::set_timestamp(12345);

		assert_ok!(ModuleOracle::feed_values(
			Origin::signed(account_id),
			vec![(1, 1000), (2, 900), (3, 800)]
		));

		assert_eq!(
			ModuleOracle::raw_values(&1, &account_id),
			Some(TimestampedValue {
				value: 1000,
				timestamp: 12345,
			})
		);

		assert_eq!(
			ModuleOracle::raw_values(&2, &account_id),
			Some(TimestampedValue {
				value: 900,
				timestamp: 12345,
			})
		);

		assert_eq!(
			ModuleOracle::raw_values(&3, &account_id),
			Some(TimestampedValue {
				value: 800,
				timestamp: 12345,
			})
		);
	});
}

#[test]
fn should_change_status_when_feeding() {
	new_test_ext().execute_with(|| {
		let key: u32 = 1;
		assert_eq!(ModuleOracle::has_update(key), false);
		assert_ok!(ModuleOracle::feed_value(Origin::signed(1), key, 1000));
		assert_eq!(ModuleOracle::has_update(key), true);
	});
}

#[test]
fn should_read_raw_values() {
	new_test_ext().execute_with(|| {
		let key: u32 = 1;

		let raw_values = ModuleOracle::read_raw_values(&key);
		assert_eq!(raw_values, vec![]);

		Timestamp::set_timestamp(12345);

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

		assert_ok!(ModuleOracle::feed_value(Origin::signed(1), key, 1000));
		assert_ok!(ModuleOracle::feed_value(Origin::signed(2), key, 1200));

		let raw_values = ModuleOracle::read_raw_values(&key);
		assert_eq!(raw_values, expected);
	});
}

#[test]
fn should_combined_data() {
	new_test_ext().execute_with(|| {
		Timestamp::set_timestamp(12345);

		let expected = Some(TimestampedValue {
			value: 1200,
			timestamp: 12345,
		});

		let key: u32 = 1;

		assert_ok!(ModuleOracle::feed_value(Origin::signed(1), key, 1300));
		assert_ok!(ModuleOracle::feed_value(Origin::signed(2), key, 1000));
		assert_ok!(ModuleOracle::feed_value(Origin::signed(3), key, 1200));
		assert_eq!(ModuleOracle::get(&key), expected);
	});
}

#[test]
fn should_return_prev_value() {
	new_test_ext().execute_with(|| {
		Timestamp::set_timestamp(12345);

		let expected = Some(TimestampedValue {
			value: 1200,
			timestamp: 12345,
		});

		let key: u32 = 1;

		assert_ok!(ModuleOracle::feed_value(Origin::signed(1), key, 1300));
		assert_ok!(ModuleOracle::feed_value(Origin::signed(2), key, 1000));
		assert_ok!(ModuleOracle::feed_value(Origin::signed(3), key, 1200));
		assert_eq!(ModuleOracle::get(&key), expected);

		Timestamp::set_timestamp(23456);

		// should return prev_value
		assert_eq!(ModuleOracle::get(&key), expected);
	});
}

#[test]
fn should_return_none() {
	new_test_ext().execute_with(|| {
		let key: u32 = 1;
		assert_eq!(ModuleOracle::get(&key), None);
	});
}

#[test]
fn should_validate() {
	new_test_ext().execute_with(|| {
		let call = Call::ModuleOracle(OracleCall::feed_values(vec![(1, 1)]));
		let info = <Call as GetDispatchInfo>::get_dispatch_info(&call);

		assert_eq!(
			CheckOperator::<Test>(Default::default()).validate(&1, &call, &info, 1),
			Ok(ValidTransaction {
				priority: TransactionPriority::max_value(),
				..Default::default()
			})
		);
	});
}

#[test]
fn should_be_free_operational() {
	new_test_ext().execute_with(|| {
		let feed_value = Call::ModuleOracle(OracleCall::feed_value(1, 1));
		let feed_values = Call::ModuleOracle(OracleCall::feed_values(vec![(1, 1)]));
		vec![feed_value, feed_values].iter().for_each(|f| {
			let dispatch_info = <Call as GetDispatchInfo>::get_dispatch_info(&f);
			assert_eq!(
				dispatch_info,
				DispatchInfo {
					weight: 0,
					class: DispatchClass::Operational,
					pays_fee: false,
				}
			);
		});
	});
}

#[test]
fn multiple_calls_should_fail() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleOracle::feed_value(Origin::signed(1), 1, 1000));
		assert_noop!(
			ModuleOracle::feed_value(Origin::signed(1), 1, 1200),
			Error::<Test>::UpdateAlreadyDispatched
		);
		<ModuleOracle as OnFinalize<u64>>::on_finalize(1);
		assert_ok!(ModuleOracle::feed_value(Origin::signed(1), 1, 1200));
	});
}

#[test]
fn get_all_values_should_work() {
	new_test_ext().execute_with(|| {
		Timestamp::set_timestamp(12345);

		let eur: u32 = 1;
		let jpy: u32 = 2;

		assert_eq!(ModuleOracle::get_all_values(), vec![]);

		// feed eur & jpy
		assert_ok!(ModuleOracle::feed_value(Origin::signed(1), eur, 1300));
		assert_ok!(ModuleOracle::feed_value(Origin::signed(2), eur, 1000));
		assert_ok!(ModuleOracle::feed_value(Origin::signed(3), jpy, 9000));

		// not enough eur & jpy prices
		assert_eq!(ModuleOracle::get(&eur), None);
		assert_eq!(ModuleOracle::get(&jpy), None);
		assert_eq!(ModuleOracle::get_all_values(), vec![]);

		// finalize block
		<ModuleOracle as OnFinalize<u64>>::on_finalize(1);

		// feed eur & jpy
		assert_ok!(ModuleOracle::feed_value(Origin::signed(3), eur, 1200));
		assert_ok!(ModuleOracle::feed_value(Origin::signed(1), jpy, 8000));

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
		assert_ok!(ModuleOracle::feed_value(Origin::signed(2), jpy, 7000));

		// enough jpy prices
		let jpy_price = Some(TimestampedValue {
			value: 8000,
			timestamp: 12345,
		});
		assert_eq!(ModuleOracle::get(&jpy), jpy_price);

		assert_eq!(ModuleOracle::get_all_values(), vec![(eur, eur_price), (jpy, jpy_price)]);
	});
}
