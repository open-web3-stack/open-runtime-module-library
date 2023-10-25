//! Unit tests for the gradually-update module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;
use parity_scale_codec::Encode;
use sp_runtime::{FixedPointNumber, FixedU128, Permill};

fn storage_set(key: &[u8], value: &[u8]) {
	// let bounded_key: StorageValueBytes<Runtime> =
	// key.to_vec().try_into().unwrap(); let bounded_value:
	// StorageValueBytes<Runtime> = key.to_vec().try_into().unwrap();
	frame_support::storage::unhashed::put(key, value);
}

fn storage_get(key: &[u8]) -> StorageValueBytes<Runtime> {
	frame_support::storage::unhashed::get::<StorageValueBytes<Runtime>>(key).unwrap_or_default()
}

#[test]
fn gradually_update_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		let update: GraduallyUpdate<StorageKeyBytes<Runtime>, StorageValueBytes<Runtime>> = GraduallyUpdate {
			key: vec![1].try_into().unwrap(),
			target_value: vec![9].try_into().unwrap(),
			per_block: vec![1].try_into().unwrap(),
		};
		assert_ok!(GraduallyUpdateModule::gradually_update(
			RuntimeOrigin::root(),
			update.clone()
		));
		System::assert_last_event(RuntimeEvent::GraduallyUpdateModule(
			crate::Event::GraduallyUpdateAdded {
				key: update.key,
				per_block: update.per_block,
				target_value: update.target_value,
			},
		));
	});
}

#[test]
fn gradually_update_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let update: GraduallyUpdate<StorageKeyBytes<Runtime>, StorageValueBytes<Runtime>> = GraduallyUpdate {
			key: vec![1].try_into().unwrap(),
			target_value: 9u32.encode().try_into().unwrap(),
			per_block: 1u64.encode().try_into().unwrap(),
		};
		assert_noop!(
			GraduallyUpdateModule::gradually_update(RuntimeOrigin::root(), update),
			Error::<Runtime>::InvalidPerBlockOrTargetValue
		);

		let update: GraduallyUpdate<StorageKeyBytes<Runtime>, StorageValueBytes<Runtime>> = GraduallyUpdate {
			key: vec![1].try_into().unwrap(),
			target_value: 90u32.encode().try_into().unwrap(),
			per_block: 1u32.encode().try_into().unwrap(),
		};
		assert_ok!(GraduallyUpdateModule::gradually_update(
			RuntimeOrigin::root(),
			update.clone()
		));

		GraduallyUpdateModule::on_finalize(20);

		let new_update = GraduallyUpdate {
			key: vec![1].try_into().unwrap(),
			target_value: 9u64.encode().try_into().unwrap(),
			per_block: 1u64.encode().try_into().unwrap(),
		};
		assert_noop!(
			GraduallyUpdateModule::gradually_update(RuntimeOrigin::root(), new_update),
			Error::<Runtime>::InvalidTargetValue
		);

		assert_noop!(
			GraduallyUpdateModule::gradually_update(RuntimeOrigin::root(), update),
			Error::<Runtime>::GraduallyUpdateHasExisted
		);
	});
}

#[test]
fn cancel_gradually_update_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		let update: GraduallyUpdate<StorageKeyBytes<Runtime>, StorageValueBytes<Runtime>> = GraduallyUpdate {
			key: vec![1].try_into().unwrap(),
			target_value: vec![9].try_into().unwrap(),
			per_block: vec![1].try_into().unwrap(),
		};
		assert_ok!(GraduallyUpdateModule::gradually_update(
			RuntimeOrigin::root(),
			update.clone()
		));
		System::assert_last_event(RuntimeEvent::GraduallyUpdateModule(
			crate::Event::GraduallyUpdateAdded {
				key: update.key.clone(),
				per_block: update.per_block,
				target_value: update.target_value,
			},
		));

		assert_ok!(GraduallyUpdateModule::cancel_gradually_update(
			RuntimeOrigin::root(),
			update.key.clone()
		));
		System::assert_last_event(RuntimeEvent::GraduallyUpdateModule(
			crate::Event::GraduallyUpdateCancelled { key: update.key },
		));
	});
}

#[test]
fn cancel_gradually_update_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let update: GraduallyUpdate<StorageKeyBytes<Runtime>, StorageValueBytes<Runtime>> = GraduallyUpdate {
			key: vec![1].try_into().unwrap(),
			target_value: 9u32.encode().try_into().unwrap(),
			per_block: 1u32.encode().try_into().unwrap(),
		};
		assert_noop!(
			GraduallyUpdateModule::cancel_gradually_update(RuntimeOrigin::root(), update.key.clone()),
			Error::<Runtime>::GraduallyUpdateNotFound
		);

		assert_ok!(GraduallyUpdateModule::gradually_update(
			RuntimeOrigin::root(),
			update.clone()
		));

		assert_ok!(GraduallyUpdateModule::cancel_gradually_update(
			RuntimeOrigin::root(),
			update.key
		));
	});
}

#[test]
fn add_on_finalize_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		let update: GraduallyUpdate<StorageKeyBytes<Runtime>, StorageValueBytes<Runtime>> = GraduallyUpdate {
			key: vec![1].try_into().unwrap(),
			target_value: vec![30].try_into().unwrap(),
			per_block: vec![1].try_into().unwrap(),
		};
		assert_ok!(GraduallyUpdateModule::gradually_update(
			RuntimeOrigin::root(),
			update.clone()
		));
		assert_eq!(storage_get(&update.key), Vec::<u8>::new());

		GraduallyUpdateModule::on_finalize(10);
		assert_eq!(storage_get(&update.key), vec![10]);
		println!("Length {}", System::events().len());
		System::assert_last_event(RuntimeEvent::GraduallyUpdateModule(crate::Event::Updated {
			block_number: 10,
			key: update.key.clone(),
			target_value: vec![10].try_into().unwrap(),
		}));
		assert_eq!(System::events().len(), 2);

		GraduallyUpdateModule::on_finalize(15);
		assert_eq!(storage_get(&update.key), vec![10]);
		assert_eq!(System::events().len(), 2);

		GraduallyUpdateModule::on_finalize(20);
		assert_eq!(storage_get(&update.key), vec![20]);
		System::assert_last_event(RuntimeEvent::GraduallyUpdateModule(crate::Event::Updated {
			block_number: 20,
			key: update.key.clone(),
			target_value: vec![20].try_into().unwrap(),
		}));
		assert_eq!(System::events().len(), 3);

		GraduallyUpdateModule::on_finalize(40);
		assert_eq!(storage_get(&update.key), vec![30]);
		System::assert_last_event(RuntimeEvent::GraduallyUpdateModule(crate::Event::Updated {
			block_number: 40,
			key: update.key,
			target_value: vec![30].try_into().unwrap(),
		}));
	});
}

#[test]
fn sub_on_finalize_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		let update: GraduallyUpdate<StorageKeyBytes<Runtime>, StorageValueBytes<Runtime>> = GraduallyUpdate {
			key: vec![1].try_into().unwrap(),
			target_value: vec![5].try_into().unwrap(),
			per_block: vec![1].try_into().unwrap(),
		};

		storage_set(&update.key, &vec![30]);
		assert_ok!(GraduallyUpdateModule::gradually_update(
			RuntimeOrigin::root(),
			update.clone()
		));
		assert_eq!(storage_get(&update.key), vec![30]);

		GraduallyUpdateModule::on_finalize(10);
		assert_eq!(storage_get(&update.key), vec![20]);
		System::assert_last_event(RuntimeEvent::GraduallyUpdateModule(crate::Event::Updated {
			block_number: 10,
			key: update.key.clone(),
			target_value: vec![20].try_into().unwrap(),
		}));
		assert_eq!(System::events().len(), 2);

		GraduallyUpdateModule::on_finalize(15);
		assert_eq!(storage_get(&update.key), vec![20]);
		assert_eq!(System::events().len(), 2);

		GraduallyUpdateModule::on_finalize(20);
		assert_eq!(storage_get(&update.key), vec![10]);
		System::assert_last_event(RuntimeEvent::GraduallyUpdateModule(crate::Event::Updated {
			block_number: 20,
			key: update.key.clone(),
			target_value: vec![10].try_into().unwrap(),
		}));
		assert_eq!(System::events().len(), 3);

		GraduallyUpdateModule::on_finalize(40);
		assert_eq!(storage_get(&update.key), vec![5]);
		System::assert_last_event(RuntimeEvent::GraduallyUpdateModule(crate::Event::Updated {
			block_number: 40,
			key: update.key,
			target_value: vec![5].try_into().unwrap(),
		}));
	});
}

#[test]
fn u32_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let update: GraduallyUpdate<StorageKeyBytes<Runtime>, StorageValueBytes<Runtime>> = GraduallyUpdate {
			key: vec![1].try_into().unwrap(),
			target_value: 30u32.encode().try_into().unwrap(),
			per_block: 1u32.encode().try_into().unwrap(),
		};
		assert_ok!(GraduallyUpdateModule::gradually_update(
			RuntimeOrigin::root(),
			update.clone()
		));
		assert_eq!(storage_get(&update.key), Vec::<u8>::new());
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
		let update: GraduallyUpdate<StorageKeyBytes<Runtime>, StorageValueBytes<Runtime>> = GraduallyUpdate {
			key: vec![1].try_into().unwrap(),
			target_value: 30u128.encode().try_into().unwrap(),
			per_block: 1u128.encode().try_into().unwrap(),
		};
		assert_ok!(GraduallyUpdateModule::gradually_update(
			RuntimeOrigin::root(),
			update.clone()
		));
		assert_eq!(storage_get(&update.key), Vec::<u8>::new());
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
		let update: GraduallyUpdate<StorageKeyBytes<Runtime>, StorageValueBytes<Runtime>> = GraduallyUpdate {
			key: vec![1].try_into().unwrap(),
			target_value: Permill::from_percent(30).encode().try_into().unwrap(),
			per_block: Permill::from_percent(1).encode().try_into().unwrap(),
		};
		assert_ok!(GraduallyUpdateModule::gradually_update(
			RuntimeOrigin::root(),
			update.clone()
		));
		assert_eq!(storage_get(&update.key), Vec::<u8>::new());
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
		let update: GraduallyUpdate<StorageKeyBytes<Runtime>, StorageValueBytes<Runtime>> = GraduallyUpdate {
			key: vec![1].try_into().unwrap(),
			target_value: FixedU128::saturating_from_rational(30, 1).encode().try_into().unwrap(),
			per_block: FixedU128::saturating_from_rational(1, 1).encode().try_into().unwrap(),
		};
		assert_ok!(GraduallyUpdateModule::gradually_update(
			RuntimeOrigin::root(),
			update.clone()
		));
		assert_eq!(storage_get(&update.key), Vec::<u8>::new());
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

#[test]
fn finish_multiple_on_finalize_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		let update: GraduallyUpdate<StorageKeyBytes<Runtime>, StorageValueBytes<Runtime>> = GraduallyUpdate {
			key: vec![10].try_into().unwrap(),
			target_value: vec![30].try_into().unwrap(),
			per_block: vec![1].try_into().unwrap(),
		};
		let update2: GraduallyUpdate<StorageKeyBytes<Runtime>, StorageValueBytes<Runtime>> = GraduallyUpdate {
			key: vec![20].try_into().unwrap(),
			target_value: vec![60].try_into().unwrap(),
			per_block: vec![2].try_into().unwrap(),
		};
		let update3: GraduallyUpdate<StorageKeyBytes<Runtime>, StorageValueBytes<Runtime>> = GraduallyUpdate {
			key: vec![30].try_into().unwrap(),
			target_value: vec![100].try_into().unwrap(),
			per_block: vec![3].try_into().unwrap(),
		};
		assert_ok!(GraduallyUpdateModule::gradually_update(
			RuntimeOrigin::root(),
			update.clone()
		));
		assert_ok!(GraduallyUpdateModule::gradually_update(
			RuntimeOrigin::root(),
			update2.clone()
		));
		assert_ok!(GraduallyUpdateModule::gradually_update(
			RuntimeOrigin::root(),
			update3.clone()
		));

		GraduallyUpdateModule::on_finalize(10);
		assert_eq!(storage_get(&update.key), vec![10]);
		assert_eq!(storage_get(&update2.key), vec![20]);
		assert_eq!(storage_get(&update3.key), vec![30]);

		GraduallyUpdateModule::on_finalize(15);
		assert_eq!(storage_get(&update.key), vec![10]);
		assert_eq!(storage_get(&update2.key), vec![20]);
		assert_eq!(storage_get(&update3.key), vec![30]);

		GraduallyUpdateModule::on_finalize(20);
		assert_eq!(storage_get(&update.key), vec![20]);
		assert_eq!(storage_get(&update2.key), vec![40]);
		assert_eq!(storage_get(&update3.key), vec![60]);

		GraduallyUpdateModule::on_finalize(40);
		assert_eq!(storage_get(&update.key), vec![30]);
		assert_eq!(storage_get(&update2.key), vec![60]);
		assert_eq!(storage_get(&update3.key), vec![90]);

		GraduallyUpdateModule::on_finalize(50);
		assert_eq!(storage_get(&update.key), vec![30]);
		assert_eq!(storage_get(&update2.key), vec![60]);
		assert_eq!(storage_get(&update3.key), vec![100]);
	});
}

#[test]
fn exceeding_max_gradually_updates_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		let update: GraduallyUpdate<StorageKeyBytes<Runtime>, StorageValueBytes<Runtime>> = GraduallyUpdate {
			key: vec![10].try_into().unwrap(),
			target_value: vec![30].try_into().unwrap(),
			per_block: vec![1].try_into().unwrap(),
		};
		let update2: GraduallyUpdate<StorageKeyBytes<Runtime>, StorageValueBytes<Runtime>> = GraduallyUpdate {
			key: vec![20].try_into().unwrap(),
			target_value: vec![60].try_into().unwrap(),
			per_block: vec![2].try_into().unwrap(),
		};
		let update3: GraduallyUpdate<StorageKeyBytes<Runtime>, StorageValueBytes<Runtime>> = GraduallyUpdate {
			key: vec![30].try_into().unwrap(),
			target_value: vec![100].try_into().unwrap(),
			per_block: vec![3].try_into().unwrap(),
		};
		let update4: GraduallyUpdate<StorageKeyBytes<Runtime>, StorageValueBytes<Runtime>> = GraduallyUpdate {
			key: vec![40].try_into().unwrap(),
			target_value: vec![120].try_into().unwrap(),
			per_block: vec![4].try_into().unwrap(),
		};
		assert_ok!(GraduallyUpdateModule::gradually_update(
			RuntimeOrigin::root(),
			update.clone()
		));
		assert_ok!(GraduallyUpdateModule::gradually_update(
			RuntimeOrigin::root(),
			update2.clone()
		));
		assert_ok!(GraduallyUpdateModule::gradually_update(
			RuntimeOrigin::root(),
			update3.clone()
		));
		assert_noop!(
			GraduallyUpdateModule::gradually_update(RuntimeOrigin::root(), update4.clone()),
			Error::<Runtime>::MaxGraduallyUpdateExceeded
		);

		GraduallyUpdateModule::on_finalize(10);
		GraduallyUpdateModule::on_finalize(20);
		GraduallyUpdateModule::on_finalize(30);
		assert_ok!(GraduallyUpdateModule::gradually_update(
			RuntimeOrigin::root(),
			update4.clone()
		));
		GraduallyUpdateModule::on_finalize(40);
		GraduallyUpdateModule::on_finalize(50);
		GraduallyUpdateModule::on_finalize(60);
		assert_eq!(storage_get(&update.key), vec![30]);
		assert_eq!(storage_get(&update2.key), vec![60]);
		assert_eq!(storage_get(&update3.key), vec![100]);
		assert_eq!(storage_get(&update4.key), vec![120]);
	});
}
