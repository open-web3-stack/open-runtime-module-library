//! Unit tests for the authority module.

#![cfg(test)]

use super::*;
use frame_support::{
	assert_noop, assert_ok,
	dispatch::DispatchErrorWithPostInfo,
	traits::{schedule::DispatchTime, OriginTrait},
};
use frame_system::RawOrigin;
use mock::{
	authority, run_to_block, Authority, BlockNumber, Call, ExtBuilder, MockAsOriginId, Origin, OriginCaller, Runtime,
	System,
};
use sp_runtime::{traits::BadOrigin, Perbill};

#[test]
fn dispatch_as_work() {
	ExtBuilder::default().build().execute_with(|| {
		let ensure_root_call = Call::System(frame_system::Call::fill_block { ratio: Perbill::one() });
		let ensure_signed_call = Call::System(frame_system::Call::remark { remark: vec![] });
		assert_ok!(Authority::dispatch_as(
			Origin::root(),
			MockAsOriginId::Root,
			Box::new(ensure_root_call)
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
				Box::new(ensure_signed_call)
			),
			BadOrigin,
		);
	});
}

#[test]
fn schedule_dispatch_at_work() {
	ExtBuilder::default().build().execute_with(|| {
		let ensure_root_call = Call::System(frame_system::Call::fill_block { ratio: Perbill::one() });
		let call = Call::Authority(authority::Call::dispatch_as {
			as_origin: MockAsOriginId::Root,
			call: Box::new(ensure_root_call),
		});
		run_to_block(1);
		assert_eq!(
			Authority::schedule_dispatch(Origin::root(), DispatchTime::At(1), 0, true, Box::new(call.clone())),
			Err(Error::<Runtime>::FailedToSchedule.into())
		);

		assert_ok!(Authority::schedule_dispatch(
			Origin::root(),
			DispatchTime::At(2),
			0,
			true,
			Box::new(call.clone())
		));
		System::assert_last_event(mock::Event::Authority(Event::Scheduled(
			OriginCaller::Authority(DelayedOrigin {
				delay: 1,
				origin: Box::new(OriginCaller::system(RawOrigin::Root)),
			}),
			1,
		)));

		run_to_block(2);
		System::assert_last_event(mock::Event::Scheduler(pallet_scheduler::Event::<Runtime>::Dispatched(
			(2, 0),
			Some([1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0].to_vec()),
			Ok(()),
		)));

		// with_delayed_origin = false
		assert_ok!(Authority::schedule_dispatch(
			Origin::root(),
			DispatchTime::At(3),
			0,
			false,
			Box::new(call)
		));
		System::assert_last_event(mock::Event::Authority(Event::Scheduled(
			OriginCaller::system(RawOrigin::Root),
			2,
		)));

		run_to_block(3);
		System::assert_last_event(mock::Event::Scheduler(pallet_scheduler::Event::<Runtime>::Dispatched(
			(3, 0),
			Some([0, 0, 2, 0, 0, 0].to_vec()),
			Ok(()),
		)));
	});
}

#[test]
fn schedule_dispatch_after_work() {
	ExtBuilder::default().build().execute_with(|| {
		let ensure_root_call = Call::System(frame_system::Call::fill_block { ratio: Perbill::one() });
		let call = Call::Authority(authority::Call::dispatch_as {
			as_origin: MockAsOriginId::Root,
			call: Box::new(ensure_root_call),
		});
		run_to_block(1);
		assert_eq!(
			Authority::schedule_dispatch(Origin::root(), DispatchTime::At(0), 0, true, Box::new(call.clone())),
			Err(ArithmeticError::Overflow.into())
		);

		assert_ok!(Authority::schedule_dispatch(
			Origin::root(),
			DispatchTime::After(0),
			0,
			true,
			Box::new(call.clone())
		));
		System::assert_last_event(mock::Event::Authority(Event::Scheduled(
			OriginCaller::Authority(DelayedOrigin {
				delay: 0,
				origin: Box::new(OriginCaller::system(RawOrigin::Root)),
			}),
			1,
		)));

		run_to_block(2);
		System::assert_last_event(mock::Event::Scheduler(pallet_scheduler::Event::<Runtime>::Dispatched(
			(2, 0),
			Some([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0].to_vec()),
			Ok(()),
		)));

		// with_delayed_origin = false
		assert_ok!(Authority::schedule_dispatch(
			Origin::root(),
			DispatchTime::After(0),
			0,
			false,
			Box::new(call)
		));
		System::assert_last_event(mock::Event::Authority(Event::Scheduled(
			OriginCaller::system(RawOrigin::Root),
			2,
		)));

		run_to_block(3);
		System::assert_last_event(mock::Event::Scheduler(pallet_scheduler::Event::<Runtime>::Dispatched(
			(3, 0),
			Some([0, 0, 2, 0, 0, 0].to_vec()),
			Ok(()),
		)));
	});
}

#[test]
fn fast_track_scheduled_dispatch_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		let ensure_root_call = Call::System(frame_system::Call::fill_block { ratio: Perbill::one() });
		let call = Call::Authority(authority::Call::dispatch_as {
			as_origin: MockAsOriginId::Root,
			call: Box::new(ensure_root_call),
		});
		run_to_block(1);
		assert_ok!(Authority::schedule_dispatch(
			Origin::root(),
			DispatchTime::At(2),
			0,
			true,
			Box::new(call.clone())
		));
		System::assert_last_event(mock::Event::Authority(Event::Scheduled(
			OriginCaller::Authority(DelayedOrigin {
				delay: 1,
				origin: Box::new(OriginCaller::system(RawOrigin::Root)),
			}),
			0,
		)));

		let schedule_origin = {
			let origin: <Runtime as Config>::Origin = Origin::root();
			let origin: <Runtime as Config>::Origin =
				From::from(DelayedOrigin::<BlockNumber, <Runtime as Config>::PalletsOrigin> {
					delay: 1,
					origin: Box::new(origin.caller().clone()),
				});
			origin
		};

		let pallets_origin = schedule_origin.caller().clone();
		assert_ok!(Authority::fast_track_scheduled_dispatch(
			Origin::root(),
			Box::new(pallets_origin),
			0,
			DispatchTime::At(4),
		));
		System::assert_last_event(mock::Event::Authority(Event::FastTracked(
			OriginCaller::Authority(DelayedOrigin {
				delay: 1,
				origin: Box::new(OriginCaller::system(RawOrigin::Root)),
			}),
			0,
			4,
		)));

		assert_ok!(Authority::schedule_dispatch(
			Origin::root(),
			DispatchTime::At(2),
			0,
			false,
			Box::new(call)
		));
		System::assert_last_event(mock::Event::Authority(Event::Scheduled(
			OriginCaller::system(RawOrigin::Root),
			1,
		)));

		assert_ok!(Authority::fast_track_scheduled_dispatch(
			Origin::root(),
			Box::new(frame_system::RawOrigin::Root.into()),
			1,
			DispatchTime::At(4),
		));
		System::assert_last_event(mock::Event::Authority(Event::FastTracked(
			OriginCaller::system(RawOrigin::Root),
			1,
			4,
		)));
	});
}

#[test]
fn delay_scheduled_dispatch_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		let ensure_root_call = Call::System(frame_system::Call::fill_block { ratio: Perbill::one() });
		let call = Call::Authority(authority::Call::dispatch_as {
			as_origin: MockAsOriginId::Root,
			call: Box::new(ensure_root_call),
		});
		run_to_block(1);
		assert_ok!(Authority::schedule_dispatch(
			Origin::root(),
			DispatchTime::At(2),
			0,
			true,
			Box::new(call.clone())
		));
		System::assert_last_event(mock::Event::Authority(Event::Scheduled(
			OriginCaller::Authority(DelayedOrigin {
				delay: 1,
				origin: Box::new(OriginCaller::system(RawOrigin::Root)),
			}),
			0,
		)));

		let schedule_origin = {
			let origin: <Runtime as Config>::Origin = Origin::root();
			let origin: <Runtime as Config>::Origin =
				From::from(DelayedOrigin::<BlockNumber, <Runtime as Config>::PalletsOrigin> {
					delay: 1,
					origin: Box::new(origin.caller().clone()),
				});
			origin
		};

		let pallets_origin = schedule_origin.caller().clone();
		assert_ok!(Authority::delay_scheduled_dispatch(
			Origin::root(),
			Box::new(pallets_origin),
			0,
			4,
		));
		System::assert_last_event(mock::Event::Authority(Event::Delayed(
			OriginCaller::Authority(DelayedOrigin {
				delay: 1,
				origin: Box::new(OriginCaller::system(RawOrigin::Root)),
			}),
			0,
			5,
		)));

		assert_ok!(Authority::schedule_dispatch(
			Origin::root(),
			DispatchTime::At(2),
			0,
			false,
			Box::new(call)
		));
		System::assert_last_event(mock::Event::Authority(Event::Scheduled(
			OriginCaller::system(RawOrigin::Root),
			1,
		)));

		assert_ok!(Authority::delay_scheduled_dispatch(
			Origin::root(),
			Box::new(frame_system::RawOrigin::Root.into()),
			1,
			4,
		));
		System::assert_last_event(mock::Event::Authority(Event::Delayed(
			OriginCaller::system(RawOrigin::Root),
			1,
			5,
		)));
	});
}

#[test]
fn cancel_scheduled_dispatch_work() {
	ExtBuilder::default().build().execute_with(|| {
		let ensure_root_call = Call::System(frame_system::Call::fill_block { ratio: Perbill::one() });
		let call = Call::Authority(authority::Call::dispatch_as {
			as_origin: MockAsOriginId::Root,
			call: Box::new(ensure_root_call),
		});
		run_to_block(1);
		assert_ok!(Authority::schedule_dispatch(
			Origin::root(),
			DispatchTime::At(2),
			0,
			true,
			Box::new(call.clone())
		));
		System::assert_last_event(mock::Event::Authority(Event::Scheduled(
			OriginCaller::Authority(DelayedOrigin {
				delay: 1,
				origin: Box::new(OriginCaller::system(RawOrigin::Root)),
			}),
			0,
		)));

		let schedule_origin = {
			let origin: <Runtime as Config>::Origin = Origin::root();
			let origin: <Runtime as Config>::Origin =
				From::from(DelayedOrigin::<BlockNumber, <Runtime as Config>::PalletsOrigin> {
					delay: 1,
					origin: Box::new(origin.caller().clone()),
				});
			origin
		};

		let pallets_origin = schedule_origin.caller().clone();
		assert_ok!(Authority::cancel_scheduled_dispatch(
			Origin::root(),
			Box::new(pallets_origin),
			0
		));
		System::assert_last_event(mock::Event::Authority(Event::Cancelled(
			OriginCaller::Authority(DelayedOrigin {
				delay: 1,
				origin: Box::new(OriginCaller::system(RawOrigin::Root)),
			}),
			0,
		)));

		assert_ok!(Authority::schedule_dispatch(
			Origin::root(),
			DispatchTime::At(2),
			0,
			false,
			Box::new(call)
		));
		System::assert_last_event(mock::Event::Authority(Event::Scheduled(
			OriginCaller::system(RawOrigin::Root),
			1,
		)));

		assert_ok!(Authority::cancel_scheduled_dispatch(
			Origin::root(),
			Box::new(frame_system::RawOrigin::Root.into()),
			1
		));
		System::assert_last_event(mock::Event::Authority(Event::Cancelled(
			OriginCaller::system(RawOrigin::Root),
			1,
		)));
	});
}

#[test]
fn call_size_limit() {
	assert!(
		core::mem::size_of::<authority::Call::<Runtime>>() <= 200,
		"size of Call is more than 200 bytes: some calls have too big arguments, use Box to \
		reduce the size of Call.
		If the limit is too strong, maybe consider increasing the limit",
	);
}

#[test]
fn authorize_call_works() {
	ExtBuilder::default().build().execute_with(|| {
		run_to_block(1);
		let ensure_root_call = Call::System(frame_system::Call::fill_block { ratio: Perbill::one() });
		let call = Call::Authority(authority::Call::dispatch_as {
			as_origin: MockAsOriginId::Root,
			call: Box::new(ensure_root_call),
		});
		let hash = <Runtime as frame_system::Config>::Hashing::hash_of(&call);

		// works without account
		assert_ok!(Authority::authorize_call(Origin::root(), Box::new(call.clone()), None));
		assert_eq!(Authority::saved_calls(&hash), Some((call.clone(), None)));
		System::assert_last_event(mock::Event::Authority(Event::AuthorizedCall(hash, None)));

		// works with account
		assert_ok!(Authority::authorize_call(
			Origin::root(),
			Box::new(call.clone()),
			Some(1)
		));
		assert_eq!(Authority::saved_calls(&hash), Some((call.clone(), Some(1))));
		System::assert_last_event(mock::Event::Authority(Event::AuthorizedCall(hash, Some(1))));
	});
}

#[test]
fn trigger_call_works() {
	ExtBuilder::default().build().execute_with(|| {
		run_to_block(1);
		let ensure_root_call = Call::System(frame_system::Call::fill_block { ratio: Perbill::one() });
		let call = Call::Authority(authority::Call::dispatch_as {
			as_origin: MockAsOriginId::Root,
			call: Box::new(ensure_root_call),
		});
		let hash = <Runtime as frame_system::Config>::Hashing::hash_of(&call);

		let call_weight_bound = call.get_dispatch_info().weight;

		// call not authorized yet
		assert_noop!(
			Authority::trigger_call(Origin::signed(1), hash, call_weight_bound),
			Error::<Runtime>::CallNotAuthorized
		);

		assert_ok!(Authority::authorize_call(Origin::root(), Box::new(call.clone()), None));

		// wrong call weight bound
		assert_noop!(
			Authority::trigger_call(Origin::signed(1), hash, call_weight_bound - 1),
			Error::<Runtime>::WrongCallWeightBound
		);

		// works without caller
		assert_ok!(Authority::trigger_call(Origin::signed(1), hash, call_weight_bound));
		assert_eq!(Authority::saved_calls(&hash), None);
		System::assert_has_event(mock::Event::Authority(Event::TriggeredCallBy(hash, 1)));
		System::assert_last_event(mock::Event::Authority(Event::Dispatched(Ok(()))));

		// works with caller 1
		assert_ok!(Authority::authorize_call(
			Origin::root(),
			Box::new(call.clone()),
			Some(1)
		));
		// caller 2 is not permitted to trigger the call
		assert_noop!(
			Authority::trigger_call(Origin::signed(2), hash, call_weight_bound),
			Error::<Runtime>::TriggerCallNotPermitted
		);
		assert_eq!(Authority::saved_calls(&hash), Some((call.clone(), Some(1))));

		// caller 1 triggering the call
		assert_ok!(Authority::trigger_call(Origin::signed(1), hash, call_weight_bound));
		assert_eq!(Authority::saved_calls(&hash), None);
		System::assert_has_event(mock::Event::Authority(Event::TriggeredCallBy(hash, 1)));
		System::assert_last_event(mock::Event::Authority(Event::Dispatched(Ok(()))));
	});
}

#[test]
fn remove_authorized_call_works() {
	ExtBuilder::default().build().execute_with(|| {
		run_to_block(1);
		let ensure_root_call = Call::System(frame_system::Call::fill_block { ratio: Perbill::one() });
		let call = Call::Authority(authority::Call::dispatch_as {
			as_origin: MockAsOriginId::Root,
			call: Box::new(ensure_root_call),
		});
		let hash = <Runtime as frame_system::Config>::Hashing::hash_of(&call);

		assert_noop!(
			Authority::remove_authorized_call(Origin::root(), hash),
			Error::<Runtime>::CallNotAuthorized
		);

		assert_ok!(Authority::authorize_call(Origin::root(), Box::new(call.clone()), None));
		assert_noop!(
			Authority::remove_authorized_call(Origin::signed(1), hash),
			Error::<Runtime>::CallNotAuthorized
		);
		assert_eq!(Authority::saved_calls(&hash), Some((call.clone(), None)));
		assert_ok!(Authority::remove_authorized_call(Origin::root(), hash));
		assert_eq!(Authority::saved_calls(&hash), None);

		assert_ok!(Authority::authorize_call(
			Origin::root(),
			Box::new(call.clone()),
			Some(1)
		));
		assert_ok!(Authority::remove_authorized_call(Origin::root(), hash));
		assert_eq!(Authority::saved_calls(&hash), None);

		assert_ok!(Authority::authorize_call(
			Origin::root(),
			Box::new(call.clone()),
			Some(1)
		));
		assert_noop!(
			Authority::remove_authorized_call(Origin::signed(2), hash),
			Error::<Runtime>::CallNotAuthorized
		);
		assert_eq!(Authority::saved_calls(&hash), Some((call.clone(), Some(1))));
		assert_ok!(Authority::remove_authorized_call(Origin::signed(1), hash));
		assert_eq!(Authority::saved_calls(&hash), None);
	});
}

#[test]
fn trigger_call_should_be_free_and_operational() {
	ExtBuilder::default().build().execute_with(|| {
		let call = Call::System(frame_system::Call::fill_block { ratio: Perbill::one() });
		let hash = <Runtime as frame_system::Config>::Hashing::hash_of(&call);
		let call_weight_bound = call.get_dispatch_info().weight;
		let trigger_call = Call::Authority(authority::Call::trigger_call {
			hash,
			call_weight_bound,
		});

		assert_ok!(Authority::authorize_call(Origin::root(), Box::new(call), Some(1)));

		// bad caller pays fee
		assert_eq!(
			trigger_call.clone().dispatch(Origin::signed(2)),
			Err(DispatchErrorWithPostInfo {
				post_info: PostDispatchInfo {
					actual_weight: None,
					pays_fee: Pays::Yes
				},
				error: Error::<Runtime>::TriggerCallNotPermitted.into()
			})
		);

		// successfull call doesn't pay fee
		assert_eq!(
			trigger_call.clone().dispatch(Origin::signed(1)),
			Ok(PostDispatchInfo {
				actual_weight: None,
				pays_fee: Pays::No
			})
		);
	});
}
