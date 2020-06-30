#![cfg(test)]

use crate::{
	mock::{new_test_ext, AccountId, ModuleOracle, OracleCall, Origin, Test, Timestamp},
	TimestampedValue,
};
use codec::Encode;
use frame_support::{
	assert_noop, assert_ok, dispatch,
	traits::{ChangeMembers, OnFinalize},
	unsigned::ValidateUnsigned,
};
use sp_runtime::{
	testing::{TestSignature, UintAuthorityId},
	transaction_validity::{InvalidTransaction, TransactionSource, TransactionValidityError},
	RuntimeAppPublic,
};

fn feed_values_from_session_key(
	id: UintAuthorityId,
	index: u32,
	nonce: u32,
	values: Vec<(u32, u32)>,
) -> Result<dispatch::DispatchResult, TransactionValidityError> {
	let now = <frame_system::Module<Test>>::block_number();
	let sig = id.sign(&(nonce, now, &values).encode()).unwrap();

	<ModuleOracle as ValidateUnsigned>::validate_unsigned(
		TransactionSource::External,
		&OracleCall::feed_values(values.clone(), index, now, sig.clone()),
	)?;

	Ok(ModuleOracle::feed_values(Origin::none(), values, index, now, sig))
}

fn feed_values(
	from: AccountId,
	index: u32,
	nonce: u32,
	values: Vec<(u32, u32)>,
) -> Result<dispatch::DispatchResult, TransactionValidityError> {
	let id = ModuleOracle::session_keys(from).unwrap();

	feed_values_from_session_key(id, index, nonce, values)
}

#[test]
fn should_feed_values() {
	new_test_ext().execute_with(|| {
		let account_id: AccountId = 1;

		assert_ok!(feed_values(account_id, 0, 0, vec![(50, 1000), (51, 900), (52, 800)]));

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
		let account_id: AccountId = 1;

		assert_ok!(ModuleOracle::feed_values(
			Origin::root(),
			vec![(50, 1000), (51, 900), (52, 800)],
			0,
			0,
			TestSignature(0, vec![])
		));

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
fn should_update_is_updated() {
	new_test_ext().execute_with(|| {
		let key: u32 = 50;
		assert_eq!(ModuleOracle::is_updated(key), false);
		assert_ok!(feed_values(1, 0, 0, vec![(key, 1000)]));
		assert_ok!(feed_values(2, 1, 0, vec![(key, 1000)]));
		assert_ok!(feed_values(3, 2, 0, vec![(key, 1000)]));
		assert_eq!(ModuleOracle::is_updated(key), false);
		assert_eq!(
			ModuleOracle::get(&key).unwrap(),
			TimestampedValue {
				value: 1000,
				timestamp: 12345
			}
		);
		assert_eq!(ModuleOracle::is_updated(key), true);
		ModuleOracle::on_finalize(1);
		assert_ok!(feed_values(1, 0, 1, vec![(key, 1000)]));
		assert_eq!(ModuleOracle::is_updated(key), false);
	});
}

#[test]
fn should_validate_index() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			feed_values(1, 1, 0, vec![(50, 1000)]),
			TransactionValidityError::Invalid(InvalidTransaction::BadProof)
		);

		assert_noop!(
			feed_values(2, 0, 0, vec![(50, 1000)]),
			TransactionValidityError::Invalid(InvalidTransaction::BadProof)
		);
	});
}

#[test]
fn should_validate_nonce() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			feed_values(1, 0, 1, vec![(50, 1000)]),
			TransactionValidityError::Invalid(InvalidTransaction::BadProof)
		);

		assert_ok!(feed_values(1, 0, 0, vec![(50, 1000)]));

		assert_eq!(ModuleOracle::nonces(&1), 1);
		ModuleOracle::on_finalize(1);

		assert_noop!(
			feed_values(1, 0, 0, vec![(50, 1000)]),
			TransactionValidityError::Invalid(InvalidTransaction::BadProof)
		);

		assert_ok!(feed_values(1, 0, 1, vec![(50, 1000)]));

		assert_eq!(ModuleOracle::nonces(&1), 2);
	});
}

#[test]
fn should_read_raw_values() {
	new_test_ext().execute_with(|| {
		let key: u32 = 50;

		let raw_values = ModuleOracle::read_raw_values(&key);
		assert_eq!(raw_values, vec![]);

		assert_ok!(feed_values(1, 0, 0, vec![(key, 1000)]));
		assert_ok!(feed_values(2, 1, 0, vec![(key, 1200)]));

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

		assert_ok!(feed_values(1, 0, 0, vec![(key, 1300)]));
		assert_ok!(feed_values(2, 1, 0, vec![(key, 1000)]));
		assert_ok!(feed_values(3, 2, 0, vec![(key, 1200)]));

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
		assert_ok!(feed_values(1, 0, 0, vec![(50, 1300)]));
		assert_noop!(
			feed_values(1, 0, 1, vec![(50, 1300)]),
			TransactionValidityError::Invalid(InvalidTransaction::Stale)
		);

		ModuleOracle::on_finalize(1);

		assert_ok!(feed_values(1, 0, 1, vec![(50, 1300)]));
	});
}

#[test]
fn get_all_values_should_work() {
	new_test_ext().execute_with(|| {
		let eur: u32 = 1;
		let jpy: u32 = 2;

		assert_eq!(ModuleOracle::get_all_values(), vec![]);

		// feed eur & jpy
		assert_ok!(feed_values(1, 0, 0, vec![(eur, 1300)]));
		assert_ok!(feed_values(2, 1, 0, vec![(eur, 1000)]));
		assert_ok!(feed_values(3, 2, 0, vec![(jpy, 9000)]));

		// not enough eur & jpy prices
		assert_eq!(ModuleOracle::get(&eur), None);
		assert_eq!(ModuleOracle::get(&jpy), None);
		assert_eq!(ModuleOracle::get_all_values(), vec![]);

		// finalize block
		ModuleOracle::on_finalize(1);

		// feed eur & jpy
		assert_ok!(feed_values(3, 2, 1, vec![(eur, 1200)]));
		assert_ok!(feed_values(1, 0, 1, vec![(jpy, 8000)]));

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
		assert_ok!(feed_values(2, 1, 1, vec![(jpy, 7000)]));

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
fn bad_index() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			feed_values(1, 255, 0, vec![(50, 1000)]),
			TransactionValidityError::Invalid(InvalidTransaction::BadProof)
		);
	});
}

#[test]
fn change_member_should_work() {
	new_test_ext().execute_with(|| {
		<ModuleOracle as ChangeMembers<AccountId>>::change_members_sorted(&[4], &[1], &[2, 3, 4]);

		assert_noop!(
			feed_values_from_session_key(10.into(), 0, 0, vec![(50, 1000)]),
			TransactionValidityError::Invalid(InvalidTransaction::BadProof)
		);

		assert_ok!(feed_values(2, 0, 0, vec![(50, 1000)]));

		assert_noop!(
			feed_values_from_session_key(40.into(), 2, 0, vec![(50, 1000)]),
			TransactionValidityError::Invalid(InvalidTransaction::BadProof)
		);

		assert_eq!(ModuleOracle::session_keys(&4), None);

		assert_ok!(ModuleOracle::set_session_key(Origin::signed(4), 40.into()));

		assert_ok!(feed_values(4, 2, 0, vec![(50, 1000)]));
	});
}

#[test]
fn should_clear_is_updated_on_change_member() {
	new_test_ext().execute_with(|| {
		assert_ok!(feed_values(1, 0, 0, vec![(50, 1000)]));
		assert_ok!(feed_values(2, 1, 0, vec![(50, 1000)]));
		assert_ok!(feed_values(3, 2, 0, vec![(50, 1000)]));

		assert_eq!(
			ModuleOracle::get(&50).unwrap(),
			TimestampedValue {
				value: 1000,
				timestamp: 12345
			}
		);
		assert_eq!(ModuleOracle::is_updated(50), true);

		ModuleOracle::change_members_sorted(&[4], &[1], &[2, 3, 4]);

		assert_eq!(ModuleOracle::is_updated(50), false);
	});
}

#[test]
fn should_clear_data_for_removed_members() {
	new_test_ext().execute_with(|| {
		assert_ok!(feed_values(1, 0, 0, vec![(50, 1000)]));
		assert_ok!(feed_values(2, 1, 0, vec![(50, 1000)]));

		ModuleOracle::change_members_sorted(&[4], &[1], &[2, 3, 4]);

		assert_eq!(ModuleOracle::raw_values(&1, 50), None);
		assert_eq!(ModuleOracle::session_keys(&1), None);
		assert_eq!(ModuleOracle::nonces(&1), 0);
	});
}

#[test]
fn change_session_key() {
	new_test_ext().execute_with(|| {
		assert_ok!(ModuleOracle::set_session_key(Origin::signed(1), 11.into()));

		assert_noop!(
			feed_values_from_session_key(10.into(), 0, 0, vec![(50, 1000)]),
			TransactionValidityError::Invalid(InvalidTransaction::BadProof)
		);

		assert_ok!(feed_values_from_session_key(11.into(), 0, 0, vec![(50, 1000)]));
	});
}
