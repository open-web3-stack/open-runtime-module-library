//! Unit tests for the authority module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{Authority, AuthorityInstance1, Call, ExtBuilder, Origin, Runtime, System};
use sp_runtime::{
	traits::{BadOrigin, Bounded},
	Perbill,
};

#[test]
fn dispatch_work() {
	ExtBuilder::default().build().execute_with(|| {
		let ensure_root_call = Call::System(frame_system::Call::fill_block(Perbill::one()));
		let ensure_signed_call = Call::System(frame_system::Call::remark(vec![]));
		assert_ok!(Authority::dispatch(
			Origin::signed(1),
			Box::new(ensure_root_call.clone())
		));
		assert_noop!(
			Authority::dispatch(Origin::signed(2), Box::new(ensure_root_call.clone())),
			BadOrigin
		);
		assert_noop!(
			Authority::dispatch(Origin::signed(1), Box::new(ensure_signed_call.clone())),
			BadOrigin
		);
		assert_noop!(
			AuthorityInstance1::dispatch(Origin::signed(1), Box::new(ensure_root_call)),
			BadOrigin
		);
		assert_ok!(AuthorityInstance1::dispatch(
			Origin::signed(1),
			Box::new(ensure_signed_call)
		));
	});
}

#[test]
fn schedule_dispatch_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(10);
		let ensure_root_call = Call::System(frame_system::Call::fill_block(Perbill::one()));

		assert_noop!(
			Authority::schedule_dispatch(
				Origin::signed(1),
				Box::new(ensure_root_call.clone()),
				DelayedDispatchTime::At(10)
			),
			Error::<Runtime, DefaultInstance>::InvalidDelayedDispatchTime,
		);
		assert_noop!(
			Authority::schedule_dispatch(
				Origin::signed(1),
				Box::new(ensure_root_call.clone()),
				DelayedDispatchTime::After(Bounded::max_value())
			),
			Error::<Runtime, DefaultInstance>::BlockNumberOverflow,
		);
		assert_noop!(
			Authority::schedule_dispatch(
				Origin::signed(2),
				Box::new(ensure_root_call.clone()),
				DelayedDispatchTime::At(20)
			),
			BadOrigin,
		);
		assert_ok!(Authority::schedule_dispatch(
			Origin::signed(1),
			Box::new(ensure_root_call.clone()),
			DelayedDispatchTime::At(20)
		));
		assert_noop!(
			Authority::schedule_dispatch(
				Origin::signed(1),
				Box::new(ensure_root_call.clone()),
				DelayedDispatchTime::At(19)
			),
			BadOrigin,
		);
		assert_ok!(Authority::schedule_dispatch(
			Origin::signed(2),
			Box::new(ensure_root_call),
			DelayedDispatchTime::At(19)
		));
	});
}

#[test]
fn schedule_dispatch_delayed_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(10);
		let call = Call::System(frame_system::Call::fill_block(Perbill::one()));

		assert_noop!(
			Authority::schedule_dispatch_delayed(
				Origin::signed(2),
				Box::new(call.clone()),
				DelayedDispatchTime::At(11)
			),
			BadOrigin,
		);

		assert_noop!(
			Authority::schedule_dispatch_delayed(Origin::signed(1), Box::new(call.clone()), DelayedDispatchTime::At(9)),
			Error::<Runtime, DefaultInstance>::InvalidDelayedDispatchTime,
		);

		assert_noop!(
			Authority::schedule_dispatch_delayed(
				Origin::signed(1),
				Box::new(call.clone()),
				DelayedDispatchTime::After(Bounded::max_value())
			),
			Error::<Runtime, DefaultInstance>::BlockNumberOverflow,
		);

		assert_ok!(Authority::schedule_dispatch_delayed(
			Origin::signed(1),
			Box::new(call.clone()),
			DelayedDispatchTime::At(11)
		));
	});
}

#[test]
fn veto_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_noop!(Authority::veto(Origin::signed(2), 1), BadOrigin,);

		assert_ok!(Authority::veto(Origin::signed(1), 1));
	});
}
