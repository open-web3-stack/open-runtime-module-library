//! Unit tests for the authority module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{Authority, Call, ExtBuilder, MockAsOriginId, Origin};
use sp_runtime::{traits::BadOrigin, Perbill};

#[test]
fn dispatch_as_work() {
	ExtBuilder::default().build().execute_with(|| {
		let ensure_root_call = Call::System(frame_system::Call::fill_block(Perbill::one()));
		let ensure_signed_call = Call::System(frame_system::Call::remark(vec![]));
		assert_ok!(Authority::dispatch_as(
			Origin::root(),
			MockAsOriginId::Root,
			Box::new(ensure_root_call.clone())
		));
		assert_ok!(Authority::dispatch_as(
			Origin::root(),
			MockAsOriginId::Account1,
			Box::new(ensure_signed_call.clone())
		));
		assert_noop!(
			Authority::dispatch_as(
				Origin::signed(1),
				MockAsOriginId::Root,
				Box::new(ensure_signed_call.clone())
			),
			BadOrigin,
		);
		assert_ok!(Authority::dispatch_as(
			Origin::signed(1),
			MockAsOriginId::Account1,
			Box::new(ensure_signed_call.clone())
		));
		assert_noop!(
			Authority::dispatch_as(
				Origin::signed(1),
				MockAsOriginId::Account2,
				Box::new(ensure_signed_call.clone())
			),
			BadOrigin,
		);
	});
}
