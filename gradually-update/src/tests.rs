//! Unit tests for the gradually-update module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{ExtBuilder, GraduallyUpdateModule, Origin, Runtime, System, TestEvent};
use orml_utilities::FixedU128;
use sp_runtime::{traits::OnFinalize, Permill};

fn storage_set(key: &Vec<u8>, value: &Vec<u8>) {
	storage::unhashed::put(key, value);
}

fn storage_get(key: &Vec<u8>) -> Vec<u8> {
	storage::unhashed::get::<StorageValue>(key).unwrap_or_default()
}

#[test]
fn gradually_update_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let update = GraduallyUpdate {
			key: vec![1],
			target_value: vec![9],
			per_block: vec![1],
		};
		assert_ok!(GraduallyUpdateModule::gradually_update(Origin::ROOT, update.clone()));

		let gradually_update_event = TestEvent::gradually_update(RawEvent::GraduallyUpdate(
			update.key,
			update.per_block,
			update.target_value,
		));
		assert!(System::events()
			.iter()
			.any(|record| record.event == gradually_update_event));
	});
}

#[test]
fn gradually_update_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let update = GraduallyUpdate {
			key: vec![1],
			target_value: 9u32.encode(),
			per_block: 1u64.encode(),
		};
		assert_noop!(
			GraduallyUpdateModule::gradually_update(Origin::ROOT, update.clone()),
			Error::<Runtime>::InvalidPerBlockOrTargetValue
		);

		let update = GraduallyUpdate {
			key: vec![1],
			target_value: 90u32.encode(),
			per_block: 1u32.encode(),
		};
		assert_ok!(GraduallyUpdateModule::gradually_update(Origin::ROOT, update.clone()));

		GraduallyUpdateModule::on_finalize(20);

		let new_update = GraduallyUpdate {
			key: vec![1],
			target_value: 9u64.encode(),
			per_block: 1u64.encode(),
		};
		assert_noop!(
			GraduallyUpdateModule::gradually_update(Origin::ROOT, new_update.clone()),
			Error::<Runtime>::InvalidTargetValue
		);

		assert_noop!(
			GraduallyUpdateModule::gradually_update(Origin::ROOT, update.clone()),
			Error::<Runtime>::GraduallyUpdateHasExisted
		);
	});
}

#[test]
fn cancel_gradually_update_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let update = GraduallyUpdate {
			key: vec![1],
			target_value: vec![9],
			per_block: vec![1],
		};
		assert_ok!(GraduallyUpdateModule::gradually_update(Origin::ROOT, update.clone()));
		let gradually_update_event = TestEvent::gradually_update(RawEvent::GraduallyUpdate(
			update.key.clone(),
			update.per_block,
			update.target_value,
		));
		assert!(System::events()
			.iter()
			.any(|record| record.event == gradually_update_event));

		assert_ok!(GraduallyUpdateModule::cancel_gradually_update(
			Origin::ROOT,
			update.key.clone()
		));
		let cancel_gradually_update_event = TestEvent::gradually_update(RawEvent::CancelGraduallyUpdate(update.key));
		assert!(System::events()
			.iter()
			.any(|record| record.event == cancel_gradually_update_event));
	});
}

#[test]
fn cancel_gradually_update_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let update = GraduallyUpdate {
			key: vec![1],
			target_value: 9u32.encode(),
			per_block: 1u32.encode(),
		};
		assert_noop!(
			GraduallyUpdateModule::cancel_gradually_update(Origin::ROOT, update.key.clone()),
			Error::<Runtime>::CancelGradullyUpdateNotExisted
		);

		assert_ok!(GraduallyUpdateModule::gradually_update(Origin::ROOT, update.clone()));

		assert_ok!(GraduallyUpdateModule::cancel_gradually_update(
			Origin::ROOT,
			update.key.clone()
		));
	});
}

#[test]
fn add_on_finalize_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let update = GraduallyUpdate {
			key: vec![1],
			target_value: vec![30],
			per_block: vec![1],
		};
		assert_ok!(GraduallyUpdateModule::gradually_update(Origin::ROOT, update.clone()));
		assert_eq!(storage_get(&update.key), vec![]);

		GraduallyUpdateModule::on_finalize(10);
		assert_eq!(storage_get(&update.key), vec![10]);
		let gradually_update_blocknumber_event =
			TestEvent::gradually_update(RawEvent::GraduallyUpdateBlockNumber(10, update.key.clone(), vec![10]));
		assert!(System::events()
			.iter()
			.any(|record| record.event == gradually_update_blocknumber_event));
		assert_eq!(System::events().len(), 2);

		GraduallyUpdateModule::on_finalize(15);
		assert_eq!(storage_get(&update.key), vec![10]);
		assert_eq!(System::events().len(), 2);

		GraduallyUpdateModule::on_finalize(20);
		assert_eq!(storage_get(&update.key), vec![20]);
		let gradually_update_blocknumber_event =
			TestEvent::gradually_update(RawEvent::GraduallyUpdateBlockNumber(20, update.key.clone(), vec![20]));
		assert!(System::events()
			.iter()
			.any(|record| record.event == gradually_update_blocknumber_event));
		assert_eq!(System::events().len(), 3);

		GraduallyUpdateModule::on_finalize(40);
		assert_eq!(storage_get(&update.key), vec![30]);
		let gradually_update_blocknumber_event =
			TestEvent::gradually_update(RawEvent::GraduallyUpdateBlockNumber(40, update.key.clone(), vec![30]));
		assert!(System::events()
			.iter()
			.any(|record| record.event == gradually_update_blocknumber_event));
	});
}

#[test]
fn sub_on_finalize_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let update = GraduallyUpdate {
			key: vec![1],
			target_value: vec![5],
			per_block: vec![1],
		};

		storage_set(&update.key, &vec![30]);
		assert_ok!(GraduallyUpdateModule::gradually_update(Origin::ROOT, update.clone()));
		assert_eq!(storage_get(&update.key), vec![30]);

		GraduallyUpdateModule::on_finalize(10);
		assert_eq!(storage_get(&update.key), vec![20]);
		let gradually_update_blocknumber_event =
			TestEvent::gradually_update(RawEvent::GraduallyUpdateBlockNumber(10, update.key.clone(), vec![20]));
		assert!(System::events()
			.iter()
			.any(|record| record.event == gradually_update_blocknumber_event));
		assert_eq!(System::events().len(), 2);

		GraduallyUpdateModule::on_finalize(15);
		assert_eq!(storage_get(&update.key), vec![20]);
		assert_eq!(System::events().len(), 2);

		GraduallyUpdateModule::on_finalize(20);
		assert_eq!(storage_get(&update.key), vec![10]);
		let gradually_update_blocknumber_event =
			TestEvent::gradually_update(RawEvent::GraduallyUpdateBlockNumber(20, update.key.clone(), vec![10]));
		assert!(System::events()
			.iter()
			.any(|record| record.event == gradually_update_blocknumber_event));
		assert_eq!(System::events().len(), 3);

		GraduallyUpdateModule::on_finalize(40);
		assert_eq!(storage_get(&update.key), vec![5]);
		let gradually_update_blocknumber_event =
			TestEvent::gradually_update(RawEvent::GraduallyUpdateBlockNumber(40, update.key.clone(), vec![5]));
		assert!(System::events()
			.iter()
			.any(|record| record.event == gradually_update_blocknumber_event));
	});
}

#[test]
fn u32_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let update = GraduallyUpdate {
			key: vec![1],
			target_value: 30u32.encode(),
			per_block: 1u32.encode(),
		};
		assert_ok!(GraduallyUpdateModule::gradually_update(Origin::ROOT, update.clone()));
		assert_eq!(storage_get(&update.key), vec![]);
		GraduallyUpdateModule::on_finalize(10);
		assert_eq!(storage_get(&update.key), vec![10, 0, 0, 0]);
		GraduallyUpdateModule::on_finalize(15);
		assert_eq!(storage_get(&update.key), vec![10, 0, 0, 0]);
		GraduallyUpdateModule::on_finalize(20);
		assert_eq!(storage_get(&update.key), vec![20, 0, 0, 0]);
		GraduallyUpdateModule::on_finalize(40);
		assert_eq!(storage_get(&update.key), vec![30, 0, 0, 0]);
	});
}

#[test]
fn u128_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let update = GraduallyUpdate {
			key: vec![1],
			target_value: 30u128.encode(),
			per_block: 1u128.encode(),
		};
		assert_ok!(GraduallyUpdateModule::gradually_update(Origin::ROOT, update.clone()));
		assert_eq!(storage_get(&update.key), vec![]);
		GraduallyUpdateModule::on_finalize(10);
		assert_eq!(
			storage_get(&update.key),
			vec![10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
		);
		GraduallyUpdateModule::on_finalize(15);
		assert_eq!(
			storage_get(&update.key),
			vec![10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
		);
		GraduallyUpdateModule::on_finalize(20);
		assert_eq!(
			storage_get(&update.key),
			vec![20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
		);
		GraduallyUpdateModule::on_finalize(40);
		assert_eq!(
			storage_get(&update.key),
			vec![30, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
		);
	});
}

#[test]
fn permill_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let update = GraduallyUpdate {
			key: vec![1],
			target_value: Permill::from_percent(30).encode(),
			per_block: Permill::from_percent(1).encode(),
		};
		assert_ok!(GraduallyUpdateModule::gradually_update(Origin::ROOT, update.clone()));
		assert_eq!(storage_get(&update.key), vec![]);
		GraduallyUpdateModule::on_finalize(10);
		assert_eq!(storage_get(&update.key), vec![160, 134, 1, 0]);
		GraduallyUpdateModule::on_finalize(15);
		assert_eq!(storage_get(&update.key), vec![160, 134, 1, 0]);
		GraduallyUpdateModule::on_finalize(20);
		assert_eq!(storage_get(&update.key), vec![64, 13, 3, 0]);
		GraduallyUpdateModule::on_finalize(40);
		assert_eq!(storage_get(&update.key), vec![224, 147, 4, 0]);
	});
}

#[test]
fn fixedu128_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let update = GraduallyUpdate {
			key: vec![1],
			target_value: FixedU128::from_rational(30, 1).encode(),
			per_block: FixedU128::from_rational(1, 1).encode(),
		};
		assert_ok!(GraduallyUpdateModule::gradually_update(Origin::ROOT, update.clone()));
		assert_eq!(storage_get(&update.key), vec![]);
		GraduallyUpdateModule::on_finalize(10);
		assert_eq!(
			storage_get(&update.key),
			vec![0, 0, 232, 137, 4, 35, 199, 138, 0, 0, 0, 0, 0, 0, 0, 0]
		);
		GraduallyUpdateModule::on_finalize(15);
		assert_eq!(
			storage_get(&update.key),
			vec![0, 0, 232, 137, 4, 35, 199, 138, 0, 0, 0, 0, 0, 0, 0, 0]
		);
		GraduallyUpdateModule::on_finalize(20);
		assert_eq!(
			storage_get(&update.key),
			vec![0, 0, 208, 19, 9, 70, 142, 21, 1, 0, 0, 0, 0, 0, 0, 0]
		);
		GraduallyUpdateModule::on_finalize(40);
		assert_eq!(
			storage_get(&update.key),
			vec![0, 0, 184, 157, 13, 105, 85, 160, 1, 0, 0, 0, 0, 0, 0, 0]
		);
	});
}
