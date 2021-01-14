//! Unit tests for the authority module.

#![cfg(test)]

use super::*;
use frame_support::{
	assert_noop, assert_ok,
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

#[test]
fn schedule_dispatch_at_work() {
	ExtBuilder::default().build().execute_with(|| {
		let ensure_root_call = Call::System(frame_system::Call::fill_block(Perbill::one()));
		let call = Call::Authority(authority::Call::dispatch_as(
			MockAsOriginId::Root,
			Box::new(ensure_root_call.clone()),
		));
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
		let event = mock::Event::authority(Event::Scheduled(
			OriginCaller::authority(DelayedOrigin {
				delay: 1,
				origin: Box::new(OriginCaller::system(RawOrigin::Root)),
			}),
			1,
		));
		assert!(System::events().iter().any(|record| record.event == event));

		run_to_block(2);
		let event = mock::Event::pallet_scheduler(pallet_scheduler::RawEvent::Dispatched(
			(2, 0),
			Some([1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0].to_vec()),
			Ok(()),
		));
		assert!(System::events().iter().any(|record| record.event == event));

		// with_delayed_origin = false
		assert_ok!(Authority::schedule_dispatch(
			Origin::root(),
			DispatchTime::At(3),
			0,
			false,
			Box::new(call.clone())
		));
		let event = mock::Event::authority(Event::Scheduled(OriginCaller::system(RawOrigin::Root), 2));
		assert!(System::events().iter().any(|record| record.event == event));

		run_to_block(3);
		let event = mock::Event::pallet_scheduler(pallet_scheduler::RawEvent::Dispatched(
			(3, 0),
			Some([0, 0, 2, 0, 0, 0].to_vec()),
			Ok(()),
		));
		assert!(System::events().iter().any(|record| record.event == event));
	});
}

#[test]
fn schedule_dispatch_after_work() {
	ExtBuilder::default().build().execute_with(|| {
		let ensure_root_call = Call::System(frame_system::Call::fill_block(Perbill::one()));
		let call = Call::Authority(authority::Call::dispatch_as(
			MockAsOriginId::Root,
			Box::new(ensure_root_call.clone()),
		));
		run_to_block(1);
		assert_eq!(
			Authority::schedule_dispatch(Origin::root(), DispatchTime::At(0), 0, true, Box::new(call.clone())),
			Err(Error::<Runtime>::Overflow.into())
		);

		assert_ok!(Authority::schedule_dispatch(
			Origin::root(),
			DispatchTime::After(0),
			0,
			true,
			Box::new(call.clone())
		));
		let event = mock::Event::authority(Event::Scheduled(
			OriginCaller::authority(DelayedOrigin {
				delay: 0,
				origin: Box::new(OriginCaller::system(RawOrigin::Root)),
			}),
			1,
		));
		assert!(System::events().iter().any(|record| record.event == event));

		run_to_block(2);
		let event = mock::Event::pallet_scheduler(pallet_scheduler::RawEvent::Dispatched(
			(2, 0),
			Some([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0].to_vec()),
			Ok(()),
		));
		assert!(System::events().iter().any(|record| record.event == event));

		// with_delayed_origin = false
		assert_ok!(Authority::schedule_dispatch(
			Origin::root(),
			DispatchTime::After(0),
			0,
			false,
			Box::new(call.clone())
		));
		let event = mock::Event::authority(Event::Scheduled(OriginCaller::system(RawOrigin::Root), 2));
		assert!(System::events().iter().any(|record| record.event == event));

		run_to_block(3);
		let event = mock::Event::pallet_scheduler(pallet_scheduler::RawEvent::Dispatched(
			(3, 0),
			Some([0, 0, 2, 0, 0, 0].to_vec()),
			Ok(()),
		));
		assert!(System::events().iter().any(|record| record.event == event));
	});
}

#[test]
fn fast_track_scheduled_dispatch_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		let ensure_root_call = Call::System(frame_system::Call::fill_block(Perbill::one()));
		let call = Call::Authority(authority::Call::dispatch_as(
			MockAsOriginId::Root,
			Box::new(ensure_root_call.clone()),
		));
		run_to_block(1);
		assert_ok!(Authority::schedule_dispatch(
			Origin::root(),
			DispatchTime::At(2),
			0,
			true,
			Box::new(call.clone())
		));
		let event = mock::Event::authority(Event::Scheduled(
			OriginCaller::authority(DelayedOrigin {
				delay: 1,
				origin: Box::new(OriginCaller::system(RawOrigin::Root)),
			}),
			0,
		));
		assert!(System::events().iter().any(|record| record.event == event));

		let schedule_origin = {
			let origin: <Runtime as Config>::Origin = From::from(Origin::root());
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
			pallets_origin,
			0,
			DispatchTime::At(4),
		));
		let event = mock::Event::authority(Event::FastTracked(
			OriginCaller::authority(DelayedOrigin {
				delay: 1,
				origin: Box::new(OriginCaller::system(RawOrigin::Root)),
			}),
			0,
			4,
		));
		assert!(System::events().iter().any(|record| record.event == event));

		assert_ok!(Authority::schedule_dispatch(
			Origin::root(),
			DispatchTime::At(2),
			0,
			false,
			Box::new(call.clone())
		));
		let event = mock::Event::authority(Event::Scheduled(OriginCaller::system(RawOrigin::Root), 1));
		assert!(System::events().iter().any(|record| record.event == event));

		assert_ok!(Authority::fast_track_scheduled_dispatch(
			Origin::root(),
			frame_system::RawOrigin::Root.into(),
			1,
			DispatchTime::At(4),
		));
		let event = mock::Event::authority(Event::FastTracked(OriginCaller::system(RawOrigin::Root), 1, 4));
		assert!(System::events().iter().any(|record| record.event == event));
	});
}

#[test]
fn delay_scheduled_dispatch_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		let ensure_root_call = Call::System(frame_system::Call::fill_block(Perbill::one()));
		let call = Call::Authority(authority::Call::dispatch_as(
			MockAsOriginId::Root,
			Box::new(ensure_root_call.clone()),
		));
		run_to_block(1);
		assert_ok!(Authority::schedule_dispatch(
			Origin::root(),
			DispatchTime::At(2),
			0,
			true,
			Box::new(call.clone())
		));
		let event = mock::Event::authority(Event::Scheduled(
			OriginCaller::authority(DelayedOrigin {
				delay: 1,
				origin: Box::new(OriginCaller::system(RawOrigin::Root)),
			}),
			0,
		));
		assert!(System::events().iter().any(|record| record.event == event));

		let schedule_origin = {
			let origin: <Runtime as Config>::Origin = From::from(Origin::root());
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
			pallets_origin,
			0,
			4,
		));
		let event = mock::Event::authority(Event::Delayed(
			OriginCaller::authority(DelayedOrigin {
				delay: 1,
				origin: Box::new(OriginCaller::system(RawOrigin::Root)),
			}),
			0,
			5,
		));
		assert!(System::events().iter().any(|record| record.event == event));

		assert_ok!(Authority::schedule_dispatch(
			Origin::root(),
			DispatchTime::At(2),
			0,
			false,
			Box::new(call.clone())
		));
		let event = mock::Event::authority(Event::Scheduled(OriginCaller::system(RawOrigin::Root), 1));
		assert!(System::events().iter().any(|record| record.event == event));

		assert_ok!(Authority::delay_scheduled_dispatch(
			Origin::root(),
			frame_system::RawOrigin::Root.into(),
			1,
			4,
		));
		let event = mock::Event::authority(Event::Delayed(OriginCaller::system(RawOrigin::Root), 1, 5));
		assert!(System::events().iter().any(|record| record.event == event));
	});
}

#[test]
fn cancel_scheduled_dispatch_work() {
	ExtBuilder::default().build().execute_with(|| {
		let ensure_root_call = Call::System(frame_system::Call::fill_block(Perbill::one()));
		let call = Call::Authority(authority::Call::dispatch_as(
			MockAsOriginId::Root,
			Box::new(ensure_root_call.clone()),
		));
		run_to_block(1);
		assert_ok!(Authority::schedule_dispatch(
			Origin::root(),
			DispatchTime::At(2),
			0,
			true,
			Box::new(call.clone())
		));
		let event = mock::Event::authority(Event::Scheduled(
			OriginCaller::authority(DelayedOrigin {
				delay: 1,
				origin: Box::new(OriginCaller::system(RawOrigin::Root)),
			}),
			0,
		));
		assert!(System::events().iter().any(|record| record.event == event));

		let schedule_origin = {
			let origin: <Runtime as Config>::Origin = From::from(Origin::root());
			let origin: <Runtime as Config>::Origin =
				From::from(DelayedOrigin::<BlockNumber, <Runtime as Config>::PalletsOrigin> {
					delay: 1,
					origin: Box::new(origin.caller().clone()),
				});
			origin
		};

		let pallets_origin = schedule_origin.caller().clone();
		assert_ok!(Authority::cancel_scheduled_dispatch(Origin::root(), pallets_origin, 0));
		let event = mock::Event::authority(Event::Cancelled(
			OriginCaller::authority(DelayedOrigin {
				delay: 1,
				origin: Box::new(OriginCaller::system(RawOrigin::Root)),
			}),
			0,
		));
		assert!(System::events().iter().any(|record| record.event == event));

		assert_ok!(Authority::schedule_dispatch(
			Origin::root(),
			DispatchTime::At(2),
			0,
			false,
			Box::new(call.clone())
		));
		let event = mock::Event::authority(Event::Scheduled(OriginCaller::system(RawOrigin::Root), 1));
		assert!(System::events().iter().any(|record| record.event == event));

		assert_ok!(Authority::cancel_scheduled_dispatch(
			Origin::root(),
			frame_system::RawOrigin::Root.into(),
			1
		));
		let event = mock::Event::authority(Event::Cancelled(OriginCaller::system(RawOrigin::Root), 1));
		assert!(System::events().iter().any(|record| record.event == event));
	});
}
