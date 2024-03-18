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
		assert_eq!(DelayTasks::next_delayed_task_id(), 0);
		assert_eq!(DelayTasks::delayed_tasks(0), None);

		assert_noop!(
			DelayTasks::add_delay_task(MockTaskType::Success(SuccessTask), 0),
			Error::<Runtime>::InvalidDelayBlock
		);

		assert_ok!(DelayTasks::add_delay_task(MockTaskType::Success(SuccessTask), 10));
		System::assert_has_event(mock::RuntimeEvent::Scheduler(
			pallet_scheduler::Event::<Runtime>::Scheduled { when: 11, index: 0 },
		));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskAdded {
			id: 0,
			task: MockTaskType::Success(SuccessTask),
			execute_block: 11,
		}));

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
		assert_eq!(
			DelayTasks::delayed_tasks(0),
			Some((MockTaskType::Success(SuccessTask), 101))
		);

		assert_noop!(
			DelayTasks::reschedule_delay_task(RuntimeOrigin::signed(1), 0, DispatchTime::At(10)),
			BadOrigin
		);

		assert_noop!(
			DelayTasks::reschedule_delay_task(RuntimeOrigin::root(), 1, DispatchTime::At(10)),
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

		System::set_block_number(100);
		assert_noop!(
			DelayTasks::reschedule_delay_task(RuntimeOrigin::root(), 0, DispatchTime::At(100)),
			Error::<Runtime>::InvalidDelayBlock
		);

		assert_noop!(
			DelayTasks::reschedule_delay_task(RuntimeOrigin::root(), 0, DispatchTime::After(0)),
			Error::<Runtime>::InvalidDelayBlock
		);

		assert_ok!(DelayTasks::reschedule_delay_task(
			RuntimeOrigin::root(),
			0,
			DispatchTime::After(1)
		));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskReDelayed {
			id: 0,
			execute_block: 101,
		}));

		assert_eq!(
			DelayTasks::delayed_tasks(0),
			Some((MockTaskType::Success(SuccessTask), 101))
		);
	});
}

#[test]
fn cancel_delayed_task_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(DelayTasks::add_delay_task(MockTaskType::Success(SuccessTask), 100));
		assert_eq!(
			DelayTasks::delayed_tasks(0),
			Some((MockTaskType::Success(SuccessTask), 101))
		);

		assert_noop!(DelayTasks::cancel_delayed_task(RuntimeOrigin::signed(1), 0), BadOrigin);

		assert_noop!(
			DelayTasks::cancel_delayed_task(RuntimeOrigin::root(), 2),
			Error::<Runtime>::InvalidId
		);

		assert_ok!(DelayTasks::cancel_delayed_task(RuntimeOrigin::root(), 0));
		System::assert_has_event(mock::RuntimeEvent::Scheduler(
			pallet_scheduler::Event::<Runtime>::Canceled { when: 101, index: 0 },
		));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskCanceled { id: 0 }));
		assert_eq!(DelayTasks::delayed_tasks(0), None);
	});
}

#[test]
fn do_delayed_execute_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(DelayTasks::do_delayed_execute(0), Error::<Runtime>::InvalidId);

		assert_ok!(DelayTasks::add_delay_task(MockTaskType::Success(SuccessTask), 100));
		assert_ok!(DelayTasks::add_delay_task(MockTaskType::Fail(FailTask), 100));

		assert_eq!(
			DelayTasks::delayed_tasks(0),
			Some((MockTaskType::Success(SuccessTask), 101))
		);
		assert_eq!(DelayTasks::delayed_tasks(1), Some((MockTaskType::Fail(FailTask), 101)));

		assert_eq!(SUCCEEDED.with(|v| *v.borrow()), 0);
		assert_eq!(
			DelayTasks::do_delayed_execute(0),
			Ok(TaskResult {
				result: Ok(()),
				used_weight: Weight::zero(),
				finished: true,
			})
		);
		assert_eq!(SUCCEEDED.with(|v| *v.borrow()), 1);
		assert_eq!(DelayTasks::delayed_tasks(0), None);

		assert_eq!(FAILED.with(|v| *v.borrow()), 0);
		assert_eq!(
			DelayTasks::do_delayed_execute(1),
			Ok(TaskResult {
				result: Err(DispatchError::Other("execute failed").into()),
				used_weight: Weight::zero(),
				finished: true,
			})
		);
		assert_eq!(FAILED.with(|v| *v.borrow()), 1);
		assert_eq!(DelayTasks::delayed_tasks(1), None);
	});
}

#[test]
fn delayed_execute_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(DelayTasks::delayed_execute(RuntimeOrigin::root(), 0), BadOrigin);

		assert_noop!(DelayTasks::delayed_execute(RuntimeOrigin::signed(1), 0), BadOrigin);

		assert_noop!(
			DelayTasks::delayed_execute(RuntimeOrigin::from(DelayedExecuteOrigin), 0),
			Error::<Runtime>::InvalidId
		);

		assert_ok!(DelayTasks::add_delay_task(MockTaskType::Success(SuccessTask), 100));
		assert_ok!(DelayTasks::add_delay_task(MockTaskType::Fail(FailTask), 100));

		assert_eq!(
			DelayTasks::delayed_tasks(0),
			Some((MockTaskType::Success(SuccessTask), 101))
		);
		assert_eq!(DelayTasks::delayed_tasks(1), Some((MockTaskType::Fail(FailTask), 101)));

		assert_eq!(SUCCEEDED.with(|v| *v.borrow()), 0);
		assert_ok!(DelayTasks::delayed_execute(
			RuntimeOrigin::from(DelayedExecuteOrigin),
			0
		));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskExecuted {
			id: 0,
			result: Ok(()),
		}));
		assert_eq!(SUCCEEDED.with(|v| *v.borrow()), 1);
		assert_eq!(DelayTasks::delayed_tasks(0), None);

		assert_eq!(FAILED.with(|v| *v.borrow()), 0);
		assert_ok!(DelayTasks::delayed_execute(
			RuntimeOrigin::from(DelayedExecuteOrigin),
			1
		));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskExecuted {
			id: 1,
			result: Err(DispatchError::Other("").into()),
		}));
		assert_eq!(FAILED.with(|v| *v.borrow()), 1);
		assert_eq!(DelayTasks::delayed_tasks(1), None);
	});
}

#[test]
fn dispatch_delayed_execute_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_ok!(DelayTasks::add_delay_task(MockTaskType::Success(SuccessTask), 10));
		assert_ok!(DelayTasks::add_delay_task(MockTaskType::Fail(FailTask), 10));

		assert_eq!(
			DelayTasks::delayed_tasks(0),
			Some((MockTaskType::Success(SuccessTask), 11))
		);
		assert_eq!(DelayTasks::delayed_tasks(1), Some((MockTaskType::Fail(FailTask), 11)));

		assert_eq!(SUCCEEDED.with(|v| *v.borrow()), 0);
		assert_eq!(FAILED.with(|v| *v.borrow()), 0);
		run_to_block(11);

		System::assert_has_event(mock::RuntimeEvent::Scheduler(
			pallet_scheduler::Event::<Runtime>::Dispatched {
				task: (11, 0),
				id: Some(blake2_256(&(&DELAY_TASK_ID, 0u64).encode())),
				result: Ok(()),
			},
		));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskExecuted {
			id: 0,
			result: Ok(()),
		}));
		System::assert_has_event(mock::RuntimeEvent::Scheduler(
			pallet_scheduler::Event::<Runtime>::Dispatched {
				task: (11, 1),
				id: Some(blake2_256(&(&DELAY_TASK_ID, 1u64).encode())),
				result: Ok(()),
			},
		));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskExecuted {
			id: 1,
			result: Err(DispatchError::Other("").into()),
		}));

		assert_eq!(SUCCEEDED.with(|v| *v.borrow()), 1);
		assert_eq!(FAILED.with(|v| *v.borrow()), 1);
		assert_eq!(DelayTasks::delayed_tasks(0), None);
		assert_eq!(DelayTasks::delayed_tasks(1), None);
	});
}
