//! Unit tests for the delay tasks.
#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;
use sp_io::hashing::blake2_256;
use sp_runtime::traits::{BadOrigin, Bounded};

#[test]
fn add_delay_task_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(
			DelayTasks::add_delay_task(MockTaskType::Success(SuccessTask), 0),
			Error::<Runtime>::InvalidDelayBlock
		);

		assert_noop!(
			DelayTasks::add_delay_task(MockTaskType::FailPreDelay(FailPreDelayTask), 0),
			Error::<Runtime>::InvalidDelayBlock
		);

		assert_eq!(PRE_DELAY_SUCCEEDED.with(|v| *v.borrow()), 0);
		assert_eq!(DelayTasks::next_delayed_task_id(), 0);
		assert_eq!(DelayTasks::delayed_tasks(0), None);
		assert_ok!(DelayTasks::add_delay_task(MockTaskType::Success(SuccessTask), 10));
		System::assert_has_event(mock::RuntimeEvent::Scheduler(
			pallet_scheduler::Event::<Runtime>::Scheduled { when: 11, index: 0 },
		));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskAdded {
			id: 0,
			task: MockTaskType::Success(SuccessTask),
			execute_block: 11,
		}));

		assert_eq!(PRE_DELAY_SUCCEEDED.with(|v| *v.borrow()), 1);
		assert_eq!(DelayTasks::next_delayed_task_id(), 1);
		assert_eq!(
			DelayTasks::delayed_tasks(0),
			Some((MockTaskType::Success(SuccessTask), 11))
		);
	});
}

#[test]
fn reschedule_delay_task_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(DelayTasks::add_delay_task(MockTaskType::Success(SuccessTask), 100));
		assert_ok!(DelayTasks::add_delay_task(
			MockTaskType::FailPreDelayedExecute(FailPreDelayedExecuteTask),
			9
		));
		assert_eq!(
			DelayTasks::delayed_tasks(0),
			Some((MockTaskType::Success(SuccessTask), 101))
		);
		assert_eq!(
			DelayTasks::delayed_tasks(1),
			Some((MockTaskType::FailPreDelayedExecute(FailPreDelayedExecuteTask), 10))
		);

		assert_noop!(
			DelayTasks::reschedule_delay_task(RuntimeOrigin::signed(ALICE), 0, DispatchTime::At(10)),
			BadOrigin
		);

		assert_noop!(
			DelayTasks::reschedule_delay_task(RuntimeOrigin::root(), 2, DispatchTime::At(10)),
			Error::<Runtime>::InvalidId
		);

		assert_noop!(
			DelayTasks::reschedule_delay_task(RuntimeOrigin::root(), 0, DispatchTime::After(Bounded::max_value())),
			ArithmeticError::Overflow
		);

		assert_ok!(DelayTasks::reschedule_delay_task(
			RuntimeOrigin::root(),
			0,
			DispatchTime::At(10)
		));
		System::assert_has_event(mock::RuntimeEvent::Scheduler(
			pallet_scheduler::Event::<Runtime>::Canceled { when: 101, index: 0 },
		));
		System::assert_has_event(mock::RuntimeEvent::Scheduler(
			pallet_scheduler::Event::<Runtime>::Scheduled { when: 10, index: 0 },
		));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskReDelayed {
			id: 0,
			execute_block: 10,
		}));

		assert_eq!(
			DelayTasks::delayed_tasks(0),
			Some((MockTaskType::Success(SuccessTask), 10))
		);

		run_to_block(10);
		assert_eq!(DelayTasks::delayed_tasks(0), None);

		// scheduler dispatched delayed_execute call for task#1,
		// but task#1 stuck for failed at pre_delayed_execute
		assert_eq!(
			DelayTasks::delayed_tasks(1),
			Some((MockTaskType::FailPreDelayedExecute(FailPreDelayedExecuteTask), 10))
		);
		System::assert_has_event(mock::RuntimeEvent::Scheduler(
			pallet_scheduler::Event::<Runtime>::Dispatched {
				task: (10, 0),
				id: Some(blake2_256(&(&DELAY_TASK_ID, 1u64).encode())),
				result: Ok(()),
			},
		));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskStuck {
			id: 1,
			error: DispatchError::Other(""),
		}));

		// cannot rescheduler stucked task
		assert_noop!(
			DelayTasks::reschedule_delay_task(RuntimeOrigin::root(), 1, DispatchTime::At(100)),
			Error::<Runtime>::FailedToSchedule
		);
	});
}

#[test]
fn cancel_delayed_task_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(DelayTasks::add_delay_task(MockTaskType::Success(SuccessTask), 100));
		assert_ok!(DelayTasks::add_delay_task(
			MockTaskType::FailPreCancel(FailPreCancelTask),
			100
		));
		assert_ok!(DelayTasks::add_delay_task(
			MockTaskType::FailPreDelayedExecute(FailPreDelayedExecuteTask),
			100
		));
		assert_eq!(
			DelayTasks::delayed_tasks(0),
			Some((MockTaskType::Success(SuccessTask), 101))
		);
		assert_eq!(
			DelayTasks::delayed_tasks(1),
			Some((MockTaskType::FailPreCancel(FailPreCancelTask), 101))
		);
		assert_eq!(
			DelayTasks::delayed_tasks(2),
			Some((MockTaskType::FailPreDelayedExecute(FailPreDelayedExecuteTask), 101))
		);

		assert_noop!(
			DelayTasks::cancel_delayed_task(RuntimeOrigin::signed(ALICE), 0, false),
			BadOrigin
		);

		assert_noop!(
			DelayTasks::cancel_delayed_task(RuntimeOrigin::root(), 3, false),
			Error::<Runtime>::InvalidId
		);

		assert_eq!(PRE_CANCEL_SUCCEEDED.with(|v| *v.borrow()), 0);
		assert_ok!(DelayTasks::cancel_delayed_task(RuntimeOrigin::root(), 0, false));
		System::assert_has_event(mock::RuntimeEvent::Scheduler(
			pallet_scheduler::Event::<Runtime>::Canceled { when: 101, index: 0 },
		));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskCanceled { id: 0 }));
		assert_eq!(DelayTasks::delayed_tasks(0), None);
		assert_eq!(PRE_CANCEL_SUCCEEDED.with(|v| *v.borrow()), 1);

		// failed cancel for failed on pre_cancel
		assert_noop!(
			DelayTasks::cancel_delayed_task(RuntimeOrigin::root(), 1, false),
			DispatchError::Other("pre_cancel failed"),
		);

		// cancel by skip pre_cancel
		assert_ok!(DelayTasks::cancel_delayed_task(RuntimeOrigin::root(), 1, true));
		System::assert_has_event(mock::RuntimeEvent::Scheduler(
			pallet_scheduler::Event::<Runtime>::Canceled { when: 101, index: 1 },
		));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskCanceled { id: 1 }));
		assert_eq!(DelayTasks::delayed_tasks(1), None);
		assert_eq!(PRE_CANCEL_SUCCEEDED.with(|v| *v.borrow()), 1); // skip pre_cancel

		run_to_block(101);

		// scheduler dispatched delayed_execute call for task#2,
		// but task#2 stuck for failed at pre_delayed_execute
		assert_eq!(
			DelayTasks::delayed_tasks(2),
			Some((MockTaskType::FailPreDelayedExecute(FailPreDelayedExecuteTask), 101))
		);
		System::assert_has_event(mock::RuntimeEvent::Scheduler(
			pallet_scheduler::Event::<Runtime>::Dispatched {
				task: (101, 2),
				id: Some(blake2_256(&(&DELAY_TASK_ID, 2u64).encode())),
				result: Ok(()),
			},
		));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskStuck {
			id: 2,
			error: DispatchError::Other(""),
		}));

		// cancel stuck task#2
		assert_ok!(DelayTasks::cancel_delayed_task(RuntimeOrigin::root(), 2, false));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskCanceled { id: 2 }));
		assert_eq!(DelayTasks::delayed_tasks(2), None);
		assert_eq!(PRE_CANCEL_SUCCEEDED.with(|v| *v.borrow()), 2);
	});
}

#[test]
fn do_delayed_execute_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(DelayTasks::delayed_execute(RuntimeOrigin::root(), 0), BadOrigin);
		assert_noop!(DelayTasks::delayed_execute(RuntimeOrigin::signed(ALICE), 0), BadOrigin);

		assert_noop!(
			DelayTasks::delayed_execute(RuntimeOrigin::from(DelayedExecuteOrigin), 0),
			Error::<Runtime>::InvalidId
		);

		assert_ok!(DelayTasks::add_delay_task(MockTaskType::Success(SuccessTask), 100));
		assert_ok!(DelayTasks::add_delay_task(
			MockTaskType::FailDispatch(FailDispatchTask),
			100
		));
		assert_ok!(DelayTasks::add_delay_task(
			MockTaskType::FailPreDelayedExecute(FailPreDelayedExecuteTask),
			100
		));

		assert_eq!(
			DelayTasks::delayed_tasks(0),
			Some((MockTaskType::Success(SuccessTask), 101))
		);
		assert_eq!(
			DelayTasks::delayed_tasks(1),
			Some((MockTaskType::FailDispatch(FailDispatchTask), 101))
		);
		assert_eq!(
			DelayTasks::delayed_tasks(2),
			Some((MockTaskType::FailPreDelayedExecute(FailPreDelayedExecuteTask), 101))
		);

		assert_eq!(DISPATCH_SUCCEEDED.with(|v| *v.borrow()), 0);
		assert_eq!(DISPATCH_FAILED.with(|v| *v.borrow()), 0);
		assert_eq!(PRE_DELAYED_EXECUTE_SUCCEEDED.with(|v| *v.borrow()), 0);

		// delayed task executed, and succeeded
		assert_ok!(DelayTasks::delayed_execute(
			RuntimeOrigin::from(DelayedExecuteOrigin),
			0
		));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskExecuted {
			id: 0,
			result: Ok(()),
		}));
		assert_eq!(DelayTasks::delayed_tasks(0), None);
		assert_eq!(DISPATCH_SUCCEEDED.with(|v| *v.borrow()), 1);
		assert_eq!(DISPATCH_FAILED.with(|v| *v.borrow()), 0);
		assert_eq!(PRE_DELAYED_EXECUTE_SUCCEEDED.with(|v| *v.borrow()), 1);

		// delayed task executed, and failed
		assert_ok!(DelayTasks::delayed_execute(
			RuntimeOrigin::from(DelayedExecuteOrigin),
			1
		));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskExecuted {
			id: 1,
			result: Err(DispatchError::Other("")),
		}));
		assert_eq!(DelayTasks::delayed_tasks(1), None);
		assert_eq!(DISPATCH_SUCCEEDED.with(|v| *v.borrow()), 1);
		assert_eq!(DISPATCH_FAILED.with(|v| *v.borrow()), 1);
		assert_eq!(PRE_DELAYED_EXECUTE_SUCCEEDED.with(|v| *v.borrow()), 2);

		// delayed task stuck for failed pre_delayed_execute
		assert_ok!(DelayTasks::delayed_execute(
			RuntimeOrigin::from(DelayedExecuteOrigin),
			2
		));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskStuck {
			id: 2,
			error: DispatchError::Other(""),
		}));
		assert_eq!(
			DelayTasks::delayed_tasks(2),
			Some((MockTaskType::FailPreDelayedExecute(FailPreDelayedExecuteTask), 101))
		);
		assert_eq!(DISPATCH_SUCCEEDED.with(|v| *v.borrow()), 1);
		assert_eq!(DISPATCH_FAILED.with(|v| *v.borrow()), 1);
		assert_eq!(PRE_DELAYED_EXECUTE_SUCCEEDED.with(|v| *v.borrow()), 2);
	});
}
