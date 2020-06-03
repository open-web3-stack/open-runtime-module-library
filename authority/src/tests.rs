//! Unit tests for the authority module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{Authority, Call, ExtBuilder, Origin, Runtime, System};
use sp_runtime::{traits::Bounded, Perbill};

#[test]
fn dispatch_root_work() {
	ExtBuilder::default().build().execute_with(|| {
		let call = Call::System(frame_system::Call::fill_block(Perbill::one()));
		assert_ok!(Authority::dispatch_root(Origin::signed(1), Box::new(call.clone())));
		assert_noop!(Authority::dispatch_root(Origin::signed(2), Box::new(call)), BadOrigin);
	});
}

#[test]
fn schedule_dispatch_root_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(10);
		let call = Call::System(frame_system::Call::fill_block(Perbill::one()));

		assert_noop!(
			Authority::schedule_dispatch_root(Origin::signed(1), Box::new(call.clone()), DelayedDispatchTime::At(10)),
			Error::<Runtime>::InvalidDelayedDispatchTime,
		);

		assert_noop!(
			Authority::schedule_dispatch_root(
				Origin::signed(1),
				Box::new(call.clone()),
				DelayedDispatchTime::After(Bounded::max_value())
			),
			Error::<Runtime>::BlockNumberOverflow,
		);

		assert_noop!(
			Authority::schedule_dispatch_root(Origin::signed(2), Box::new(call.clone()), DelayedDispatchTime::At(20)),
			BadOrigin,
		);
		assert_ok!(Authority::schedule_dispatch_root(
			Origin::signed(1),
			Box::new(call.clone()),
			DelayedDispatchTime::At(20)
		));

		assert_noop!(
			Authority::schedule_dispatch_root(Origin::signed(1), Box::new(call.clone()), DelayedDispatchTime::At(19)),
			BadOrigin,
		);
		assert_ok!(Authority::schedule_dispatch_root(
			Origin::signed(2),
			Box::new(call.clone()),
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
			Error::<Runtime>::InvalidDelayedDispatchTime,
		);

		assert_noop!(
			Authority::schedule_dispatch_delayed(
				Origin::signed(1),
				Box::new(call.clone()),
				DelayedDispatchTime::After(Bounded::max_value())
			),
			Error::<Runtime>::BlockNumberOverflow,
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
