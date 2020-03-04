//! Unit tests for the gradually-update module.

#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::{BalancesCall, Call, ExtBuilder, Origin, Runtime, ScheduleUpdateModule, System, TestEvent};
use sp_runtime::traits::OnInitialize;

#[test]
fn schedule_dispatch_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		// NormalDispatches
		let call = Call::Balances(BalancesCall::transfer(2, 110));
		assert_ok!(ScheduleUpdateModule::schedule_dispatch(
			Origin::signed(1),
			call,
			DelayedDispatchTime::At(2)
		));

		let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::ScheduleDispatch(1, 0, 2));
		assert!(System::events()
			.iter()
			.any(|record| record.event == schedule_dispatch_event));

		// OperationalDispatches
		//let call = Call::Balances(BalancesCall::set_balance(1, 10, 110));
		//assert_ok!(ScheduleUpdateModule::schedule_dispatch(Origin::ROOT, call, DelayedDispatchTime::After(3)));

		//let schedule_dispatch_event = TestEvent::schedule_update(RawEvent::ScheduleDispatch(1, 0, 2,));
		//assert!(System::events().iter().any(|record| record.event == schedule_dispatch_event));
	});
}
