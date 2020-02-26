//! Unit tests for the gradually-update module.

#![cfg(test)]

use super::*;
use frame_support::assert_ok;
use mock::{ExtBuilder, GraduallyUpdateModule, Origin, ALICE};
use sp_runtime::traits::OnFinalize;

#[test]
fn gradually_update_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let update = GraduallyUpdate {
			key: vec![],
			target_value: vec![],
			per_block: vec![],
		};
		assert_ok!(GraduallyUpdateModule::gradually_update(Origin::ROOT, update));

		//println!("{:?}", <Module<mock::Runtime> as Trait>::GraduallyUpdates::<mock::Runtime>::hashed_key());
		//let value = storage::unhashed::get::<StorageValue>(&update.key).unwrap_or_default();
		//println!("{:?}", value);
	});
}

#[test]
fn cancel_gradually_update_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		let update = GraduallyUpdate {
			key: vec![],
			target_value: vec![],
			per_block: vec![],
		};
		assert_ok!(GraduallyUpdateModule::gradually_update(Origin::ROOT, update.clone()));

		assert_ok!(GraduallyUpdateModule::cancel_gradually_update(Origin::ROOT, update.key));
	});
}

#[test]
fn on_finalize_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		GraduallyUpdateModule::on_finalize(50);
	});
}
