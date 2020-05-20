//! Unit tests for the gradually-update module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok, traits::OnInitialize};
use mock::{BalancesCall, Call, ExtBuilder, Origin, Runtime, ScheduleUpdateModule, System, TestEvent, ALICE, BOB};

#[test]
fn schedule_dispatch_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// NormalDispatches
		let call = Call::Balances(BalancesCall::transfer(2, 11));
		assert_ok!(ScheduleUpdateModule::schedule_dispatch(
			Origin::signed(ALICE),
			Box::new(call),
			DelayedDispatchTime::At(2)
		));

		let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::ScheduleDispatch(2, 0));
		assert!(System::events()
			.iter()
			.any(|record| record.event == schedule_dispatch_event));

		// OperationalDispatches
		let call = Call::Balances(BalancesCall::set_balance(1, 10, 11));
		assert_ok!(ScheduleUpdateModule::schedule_dispatch(
			Origin::ROOT,
			Box::new(call),
			DelayedDispatchTime::After(3)
		));

		let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::ScheduleDispatch(4, 1));
		assert!(System::events()
			.iter()
			.any(|record| record.event == schedule_dispatch_event));
	});
}

#[test]
fn schedule_dispatch_works_for_root_origin() {
	ExtBuilder::default().build().execute_with(|| {
		let call = Call::Balances(BalancesCall::transfer(2, 11));
		assert_ok!(ScheduleUpdateModule::schedule_dispatch(
			Origin::ROOT,
			Box::new(call),
			DelayedDispatchTime::At(10)
		));
	});
}

#[test]
fn schedule_dispatch_fails_if_not_allowed_origin() {
	ExtBuilder::default().build().execute_with(|| {
		let call = Call::Balances(BalancesCall::transfer(2, 11));
		assert_noop!(
			ScheduleUpdateModule::schedule_dispatch(Origin::signed(BOB), Box::new(call), DelayedDispatchTime::At(10)),
			DispatchError::BadOrigin,
		);
	});
}

#[test]
fn schedule_dispatch_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		let call = Call::Balances(BalancesCall::transfer(2, 11));
		assert_noop!(
			ScheduleUpdateModule::schedule_dispatch(Origin::signed(ALICE), Box::new(call), DelayedDispatchTime::At(0)),
			Error::<Runtime>::InvalidDelayedDispatchTime
		);
	});
}

#[test]
fn cancel_delayed_dispatch_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// NormalDispatches
		let call = Call::Balances(BalancesCall::transfer(2, 11));
		assert_ok!(ScheduleUpdateModule::schedule_dispatch(
			Origin::signed(ALICE),
			Box::new(call),
			DelayedDispatchTime::At(2)
		));

		let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::ScheduleDispatch(2, 0));
		assert!(System::events()
			.iter()
			.any(|record| record.event == schedule_dispatch_event));

		assert_ok!(ScheduleUpdateModule::cancel_delayed_dispatch(
			Origin::signed(ALICE),
			2,
			0
		));

		let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::CancelDelayedDispatch(0));
		assert!(System::events()
			.iter()
			.any(|record| record.event == schedule_dispatch_event));

		// root cancel NormalDispatches
		let call = Call::Balances(BalancesCall::transfer(2, 12));
		assert_ok!(ScheduleUpdateModule::schedule_dispatch(
			Origin::signed(ALICE),
			Box::new(call),
			DelayedDispatchTime::After(3)
		));

		let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::ScheduleDispatch(4, 1));
		assert!(System::events()
			.iter()
			.any(|record| record.event == schedule_dispatch_event));

		assert_ok!(ScheduleUpdateModule::cancel_delayed_dispatch(Origin::ROOT, 4, 1));

		let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::CancelDelayedDispatch(1));
		assert!(System::events()
			.iter()
			.any(|record| record.event == schedule_dispatch_event));

		// OperationalDispatches
		let call = Call::Balances(BalancesCall::set_balance(2, 10, 13));
		assert_ok!(ScheduleUpdateModule::schedule_dispatch(
			Origin::ROOT,
			Box::new(call),
			DelayedDispatchTime::At(5)
		));

		let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::ScheduleDispatch(5, 2));
		assert!(System::events()
			.iter()
			.any(|record| record.event == schedule_dispatch_event));

		assert_ok!(ScheduleUpdateModule::cancel_delayed_dispatch(Origin::ROOT, 5, 2));

		let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::CancelDelayedDispatch(2));
		assert!(System::events()
			.iter()
			.any(|record| record.event == schedule_dispatch_event));
	});
}

#[test]
fn cancel_delayed_dispatch_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			ScheduleUpdateModule::cancel_delayed_dispatch(Origin::signed(ALICE), 2, 0),
			Error::<Runtime>::DispatchNotExisted
		);

		// NormalDispatches
		let call = Call::Balances(BalancesCall::transfer(2, 11));
		assert_ok!(ScheduleUpdateModule::schedule_dispatch(
			Origin::signed(ALICE),
			Box::new(call),
			DelayedDispatchTime::At(2)
		));

		let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::ScheduleDispatch(2, 0));
		assert!(System::events()
			.iter()
			.any(|record| record.event == schedule_dispatch_event));

		assert_noop!(
			ScheduleUpdateModule::cancel_delayed_dispatch(Origin::signed(BOB), 2, 0),
			Error::<Runtime>::NoPermission
		);

		// OperationalDispatches
		let call = Call::Balances(BalancesCall::set_balance(2, 10, 13));
		assert_ok!(ScheduleUpdateModule::schedule_dispatch(
			Origin::ROOT,
			Box::new(call),
			DelayedDispatchTime::At(5)
		));

		let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::ScheduleDispatch(5, 1));
		assert!(System::events()
			.iter()
			.any(|record| record.event == schedule_dispatch_event));

		assert_noop!(
			ScheduleUpdateModule::cancel_delayed_dispatch(Origin::signed(BOB), 5, 1),
			Error::<Runtime>::NoPermission
		);
	});
}

#[test]
fn on_initialize_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// NormalDispatches
		let call = Call::Balances(BalancesCall::transfer(2, 11));
		assert_ok!(ScheduleUpdateModule::schedule_dispatch(
			Origin::signed(ALICE),
			Box::new(call),
			DelayedDispatchTime::At(2)
		));

		let call = Call::Balances(BalancesCall::transfer(2, 12));
		assert_ok!(ScheduleUpdateModule::schedule_dispatch(
			Origin::signed(ALICE),
			Box::new(call),
			DelayedDispatchTime::At(3)
		));

		assert_eq!(System::events().len(), 2);
		ScheduleUpdateModule::on_initialize(1);
		assert_eq!(System::events().len(), 2);

		ScheduleUpdateModule::on_initialize(2);
		println!("{:?}", System::events());
		assert_eq!(System::events().len(), 4);
		let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::ScheduleDispatchSuccess(2, 0));
		assert!(System::events()
			.iter()
			.any(|record| record.event == schedule_dispatch_event));

		ScheduleUpdateModule::on_initialize(3);
		assert_eq!(System::events().len(), 6);
		let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::ScheduleDispatchSuccess(3, 1));
		assert!(System::events()
			.iter()
			.any(|record| record.event == schedule_dispatch_event));

		// OperationalDispatches
		let call = Call::Balances(BalancesCall::set_balance(3, 10, 11));
		assert_ok!(ScheduleUpdateModule::schedule_dispatch(
			Origin::ROOT,
			Box::new(call),
			DelayedDispatchTime::After(10)
		));

		let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::ScheduleDispatch(11, 2));
		assert!(System::events()
			.iter()
			.any(|record| record.event == schedule_dispatch_event));

		let call = Call::Balances(BalancesCall::set_balance(3, 20, 21));
		assert_ok!(ScheduleUpdateModule::schedule_dispatch(
			Origin::ROOT,
			Box::new(call),
			DelayedDispatchTime::After(12)
		));

		let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::ScheduleDispatch(13, 3));
		assert!(System::events()
			.iter()
			.any(|record| record.event == schedule_dispatch_event));

		assert_eq!(System::events().len(), 8);
		ScheduleUpdateModule::on_initialize(10);
		assert_eq!(System::events().len(), 8);

		ScheduleUpdateModule::on_initialize(11);
		println!("{:?}", System::events());
		assert_eq!(System::events().len(), 10);
		let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::ScheduleDispatchSuccess(11, 2));
		assert!(System::events()
			.iter()
			.any(|record| record.event == schedule_dispatch_event));

		ScheduleUpdateModule::on_initialize(13);
		assert_eq!(System::events().len(), 12);
		let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::ScheduleDispatchSuccess(13, 3));
		assert!(System::events()
			.iter()
			.any(|record| record.event == schedule_dispatch_event));
	});
}

#[test]
fn on_initialize_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// NormalDispatches balance not enough
		let call = Call::Balances(BalancesCall::transfer(2, 110));
		assert_ok!(ScheduleUpdateModule::schedule_dispatch(
			Origin::signed(ALICE),
			Box::new(call),
			DelayedDispatchTime::At(2)
		));

		assert_eq!(System::events().len(), 1);
		ScheduleUpdateModule::on_initialize(1);
		assert_eq!(System::events().len(), 1);

		ScheduleUpdateModule::on_initialize(2);
		println!("{:?}", System::events());
		assert_eq!(System::events().len(), 2);
		//TODO hold the error
		let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::ScheduleDispatchFail(
			0,
			DispatchError::Module {
				index: 0,
				error: 3,
				message: None,
			},
		));
		assert!(System::events()
			.iter()
			.any(|record| record.event == schedule_dispatch_event));

		// OperationalDispatches not root
		let call = Call::Balances(BalancesCall::set_balance(3, 10, 11));
		assert_ok!(ScheduleUpdateModule::schedule_dispatch(
			Origin::signed(ALICE),
			Box::new(call),
			DelayedDispatchTime::After(10)
		));

		assert_eq!(System::events().len(), 3);
		ScheduleUpdateModule::on_initialize(10);
		assert_eq!(System::events().len(), 3);

		ScheduleUpdateModule::on_initialize(11);
		println!("{:?}", System::events());
		assert_eq!(System::events().len(), 4);
		let schedule_dispatch_event =
			TestEvent::schedule_update(RawEvent::ScheduleDispatchFail(1, DispatchError::BadOrigin));
		assert!(System::events()
			.iter()
			.any(|record| record.event == schedule_dispatch_event));
	});
}

#[test]
fn on_initialize_weight_exceed() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		// NormalDispatches
		let call = Call::Balances(BalancesCall::transfer(2, 11));
		assert_ok!(ScheduleUpdateModule::schedule_dispatch(
			Origin::signed(ALICE),
			Box::new(call),
			DelayedDispatchTime::At(2)
		));

		let call = Call::Balances(BalancesCall::transfer(2, 12));
		assert_ok!(ScheduleUpdateModule::schedule_dispatch(
			Origin::signed(ALICE),
			Box::new(call),
			DelayedDispatchTime::At(2)
		));

		let call = Call::Balances(BalancesCall::transfer(2, 13));
		assert_ok!(ScheduleUpdateModule::schedule_dispatch(
			Origin::signed(ALICE),
			Box::new(call),
			DelayedDispatchTime::At(2)
		));

		assert_eq!(System::events().len(), 3);
		ScheduleUpdateModule::on_initialize(1);
		assert_eq!(System::events().len(), 3);

		ScheduleUpdateModule::on_initialize(2);
		println!("{:?}", System::events());
		assert_eq!(System::events().len(), 7);
		// TODO on_initialize should be sorted
		//let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::ScheduleDispatchSuccess(0, 2));
		//assert!(System::events().iter().any(|record| record.event == schedule_dispatch_event));

		//let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::ScheduleDispatchSuccess(2, 2));
		//assert!(System::events().iter().any(|record| record.event == schedule_dispatch_event));

		ScheduleUpdateModule::on_initialize(3);
		assert_eq!(System::events().len(), 9);
		//let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::ScheduleDispatchSuccess(1, 3));
		//assert!(System::events().iter().any(|record| record.event == schedule_dispatch_event));
	});
}
