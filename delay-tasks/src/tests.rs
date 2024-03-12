//! Unit tests for the delay tasks.
#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;
use sp_runtime::traits::{BadOrigin, Bounded};

#[test]
fn add_delay_task_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		assert_eq!(DelayTasks::next_delayed_task_id(), 0);
		assert_eq!(DelayTasks::delayed_tasks(0), None);

		assert_noop!(
			DelayTasks::add_delay_task(MockDelayedTaskType::Success(SuccessTask), 0),
			Error::<Runtime>::InvalidDelayBlock
		);
		assert_eq!(PRE_DELAYED.with(|v| *v.borrow()), false);

		assert_ok!(DelayTasks::add_delay_task(
			MockDelayedTaskType::Success(SuccessTask),
			100
		));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskAdded {
			id: 0,
			task: MockDelayedTaskType::Success(SuccessTask),
			execute_block: 101,
		}));

		assert_eq!(DelayTasks::next_delayed_task_id(), 1);
		assert_eq!(
			DelayTasks::delayed_tasks(0),
			Some((MockDelayedTaskType::Success(SuccessTask), 101))
		);
		assert_eq!(DelayTasks::delayed_task_queue(101, 0), Some(()));
		assert_eq!(PRE_DELAYED.with(|v| *v.borrow()), true);

		reset_delay_process_records();
		assert_noop!(
			DelayTasks::add_delay_task(MockDelayedTaskType::FailedPreDelay(FailedPreDelayTask), 200),
			DispatchError::Other("pre_delay failed")
		);
		assert_eq!(PRE_DELAYED.with(|v| *v.borrow()), false);
	});
}

#[test]
fn reset_execute_block_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(DelayTasks::add_delay_task(
			MockDelayedTaskType::Success(SuccessTask),
			100
		));
		assert_eq!(
			DelayTasks::delayed_tasks(0),
			Some((MockDelayedTaskType::Success(SuccessTask), 101))
		);
		assert_eq!(DelayTasks::delayed_task_queue(101, 0), Some(()));
		assert_eq!(DelayTasks::delayed_task_queue(10, 0), None);

		assert_noop!(
			DelayTasks::reset_execute_block(RuntimeOrigin::signed(1), 0, DispatchTime::At(10)),
			BadOrigin
		);

		assert_noop!(
			DelayTasks::reset_execute_block(RuntimeOrigin::root(), 1, DispatchTime::At(10)),
			Error::<Runtime>::InvalidId
		);

		assert_noop!(
			DelayTasks::reset_execute_block(RuntimeOrigin::root(), 0, DispatchTime::After(Bounded::max_value())),
			ArithmeticError::Overflow
		);

		assert_ok!(DelayTasks::reset_execute_block(
			RuntimeOrigin::root(),
			0,
			DispatchTime::At(10)
		));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskReDelayed {
			id: 0,
			execute_block: 10,
		}));

		assert_eq!(
			DelayTasks::delayed_tasks(0),
			Some((MockDelayedTaskType::Success(SuccessTask), 10))
		);
		assert_eq!(DelayTasks::delayed_task_queue(101, 0), None);
		assert_eq!(DelayTasks::delayed_task_queue(10, 0), Some(()));

		System::set_block_number(100);
		assert_noop!(
			DelayTasks::reset_execute_block(RuntimeOrigin::root(), 0, DispatchTime::At(100)),
			Error::<Runtime>::InvalidDelayBlock
		);

		assert_noop!(
			DelayTasks::reset_execute_block(RuntimeOrigin::root(), 0, DispatchTime::After(0)),
			Error::<Runtime>::InvalidDelayBlock
		);

		assert_ok!(DelayTasks::reset_execute_block(
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
			Some((MockDelayedTaskType::Success(SuccessTask), 101))
		);
		assert_eq!(DelayTasks::delayed_task_queue(101, 0), Some(()));
		assert_eq!(DelayTasks::delayed_task_queue(10, 0), None);
	});
}

#[test]
fn cancel_delayed_task_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(DelayTasks::add_delay_task(
			MockDelayedTaskType::Success(SuccessTask),
			100
		));
		assert_ok!(DelayTasks::add_delay_task(
			MockDelayedTaskType::FailedOnCancel(FailedOnCancelTask),
			200
		));

		assert_eq!(
			DelayTasks::delayed_tasks(0),
			Some((MockDelayedTaskType::Success(SuccessTask), 101))
		);
		assert_eq!(DelayTasks::delayed_task_queue(101, 0), Some(()));
		assert_eq!(
			DelayTasks::delayed_tasks(1),
			Some((MockDelayedTaskType::FailedOnCancel(FailedOnCancelTask), 201))
		);
		assert_eq!(DelayTasks::delayed_task_queue(201, 1), Some(()));

		assert_noop!(DelayTasks::cancel_delayed_task(RuntimeOrigin::signed(1), 0), BadOrigin);

		assert_noop!(
			DelayTasks::cancel_delayed_task(RuntimeOrigin::root(), 2),
			Error::<Runtime>::InvalidId
		);

		assert_ok!(DelayTasks::cancel_delayed_task(RuntimeOrigin::root(), 0));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskCanceled { id: 0 }));
		assert_eq!(ON_CANCEL.with(|v| *v.borrow()), true);
		assert_eq!(DelayTasks::delayed_tasks(0), None);
		assert_eq!(DelayTasks::delayed_task_queue(101, 0), None);

		reset_delay_process_records();
		assert_noop!(
			DelayTasks::cancel_delayed_task(RuntimeOrigin::root(), 1),
			DispatchError::Other("on_cancel failed")
		);
		assert_eq!(ON_CANCEL.with(|v| *v.borrow()), false);
		assert_eq!(
			DelayTasks::delayed_tasks(1),
			Some((MockDelayedTaskType::FailedOnCancel(FailedOnCancelTask), 201))
		);
		assert_eq!(DelayTasks::delayed_task_queue(201, 1), Some(()));
	});
}

#[test]
fn do_execute_delayed_task_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);

		assert_noop!(DelayTasks::do_execute_delayed_task(0), Error::<Runtime>::InvalidId);

		assert_ok!(DelayTasks::add_delay_task(
			MockDelayedTaskType::Success(SuccessTask),
			100
		));
		assert_ok!(DelayTasks::add_delay_task(
			MockDelayedTaskType::FailedPreDelayedExecute(FailedPreDelayedExecuteTask),
			100
		));
		assert_ok!(DelayTasks::add_delay_task(
			MockDelayedTaskType::FailedDelayedExecute(FailedDelayedExecuteTask),
			100
		));
		assert_ok!(DelayTasks::add_delay_task(
			MockDelayedTaskType::FailedOnCancel(FailedOnCancelTask),
			100
		));

		assert_eq!(
			DelayTasks::delayed_tasks(0),
			Some((MockDelayedTaskType::Success(SuccessTask), 101))
		);
		assert_eq!(
			DelayTasks::delayed_tasks(1),
			Some((
				MockDelayedTaskType::FailedPreDelayedExecute(FailedPreDelayedExecuteTask),
				101
			))
		);
		assert_eq!(
			DelayTasks::delayed_tasks(2),
			Some((MockDelayedTaskType::FailedDelayedExecute(FailedDelayedExecuteTask), 101))
		);
		assert_eq!(
			DelayTasks::delayed_tasks(3),
			Some((MockDelayedTaskType::FailedOnCancel(FailedOnCancelTask), 101))
		);
		assert_eq!(DelayTasks::delayed_task_queue(101, 0), Some(()));
		assert_eq!(DelayTasks::delayed_task_queue(101, 1), Some(()));
		assert_eq!(DelayTasks::delayed_task_queue(101, 2), Some(()));
		assert_eq!(DelayTasks::delayed_task_queue(101, 3), Some(()));

		// execute delayed task
		reset_delay_process_records();
		assert_ok!(DelayTasks::do_execute_delayed_task(0));
		assert_eq!(PRE_DELAYED_EXECUTED.with(|v| *v.borrow()), true);
		assert_eq!(EXECUTED.with(|v| *v.borrow()), true);
		assert_eq!(DelayTasks::delayed_tasks(0), None);
		assert_eq!(DelayTasks::delayed_task_queue(101, 0), Some(())); // do_execute_delayed_task doesn't clear delayed_task_queue

		// failed execute delayed task for failed pre_delayed_execute
		reset_delay_process_records();
		assert_noop!(
			DelayTasks::do_execute_delayed_task(1),
			DispatchError::Other("pre_delayed_execute failed")
		);
		assert_eq!(PRE_DELAYED_EXECUTED.with(|v| *v.borrow()), false);
		assert_eq!(EXECUTED.with(|v| *v.borrow()), false);
		assert_eq!(
			DelayTasks::delayed_tasks(1),
			Some((
				MockDelayedTaskType::FailedPreDelayedExecute(FailedPreDelayedExecuteTask),
				101
			))
		);
		assert_eq!(DelayTasks::delayed_task_queue(101, 1), Some(())); // do_execute_delayed_task doesn't clear delayed_task_queue

		// execute delayed task but task failed
		reset_delay_process_records();
		assert_eq!(
			DelayTasks::do_execute_delayed_task(2),
			Ok(Err(DispatchError::Other("delayed_execute failed")))
		);
		assert_eq!(PRE_DELAYED_EXECUTED.with(|v| *v.borrow()), true);
		assert_eq!(EXECUTED.with(|v| *v.borrow()), false);
		assert_eq!(DelayTasks::delayed_tasks(2), None);
		assert_eq!(DelayTasks::delayed_task_queue(101, 2), Some(())); // do_execute_delayed_task doesn't clear delayed_task_queue

		// execute delayed task
		reset_delay_process_records();
		assert_ok!(DelayTasks::do_execute_delayed_task(3));
		assert_eq!(PRE_DELAYED_EXECUTED.with(|v| *v.borrow()), true);
		assert_eq!(EXECUTED.with(|v| *v.borrow()), true);
		assert_eq!(DelayTasks::delayed_tasks(3), None);
		assert_eq!(DelayTasks::delayed_task_queue(101, 3), Some(())); // do_execute_delayed_task doesn't clear delayed_task_queue
	});
}

#[test]
fn on_finalize_work() {
	ExtBuilder::default().build().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(DelayTasks::add_delay_task(
			MockDelayedTaskType::Success(SuccessTask),
			100
		));
		assert_ok!(DelayTasks::add_delay_task(
			MockDelayedTaskType::FailedPreDelayedExecute(FailedPreDelayedExecuteTask),
			100
		));
		System::set_block_number(2);
		assert_ok!(DelayTasks::add_delay_task(
			MockDelayedTaskType::FailedDelayedExecute(FailedDelayedExecuteTask),
			100
		));
		assert_ok!(DelayTasks::add_delay_task(
			MockDelayedTaskType::FailedOnCancel(FailedOnCancelTask),
			100
		));

		assert_eq!(
			DelayTasks::delayed_tasks(0),
			Some((MockDelayedTaskType::Success(SuccessTask), 101))
		);
		assert_eq!(
			DelayTasks::delayed_tasks(1),
			Some((
				MockDelayedTaskType::FailedPreDelayedExecute(FailedPreDelayedExecuteTask),
				101
			))
		);
		assert_eq!(
			DelayTasks::delayed_tasks(2),
			Some((MockDelayedTaskType::FailedDelayedExecute(FailedDelayedExecuteTask), 102))
		);
		assert_eq!(
			DelayTasks::delayed_tasks(3),
			Some((MockDelayedTaskType::FailedOnCancel(FailedOnCancelTask), 102))
		);
		assert_eq!(DelayTasks::delayed_task_queue(101, 0), Some(()));
		assert_eq!(DelayTasks::delayed_task_queue(101, 1), Some(()));
		assert_eq!(DelayTasks::delayed_task_queue(102, 2), Some(()));
		assert_eq!(DelayTasks::delayed_task_queue(102, 3), Some(()));

		DelayTasks::on_finalize(101);
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskExecuted {
			id: 0,
			result: Ok(()),
		}));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskTryExecuteFailed {
			id: 1,
			error: DispatchError::Other(""),
		}));
		assert_eq!(DelayTasks::delayed_tasks(0), None);
		assert_eq!(
			DelayTasks::delayed_tasks(1),
			Some((
				MockDelayedTaskType::FailedPreDelayedExecute(FailedPreDelayedExecuteTask),
				101
			))
		); // keep the storage for failed executed delayed task
		assert_eq!(DelayTasks::delayed_task_queue(101, 0), None);
		assert_eq!(DelayTasks::delayed_task_queue(101, 1), None);

		DelayTasks::on_finalize(102);
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskExecuted {
			id: 2,
			result: Err(DispatchError::Other("").into()),
		}));
		System::assert_has_event(mock::RuntimeEvent::DelayTasks(Event::DelayedTaskExecuted {
			id: 3,
			result: Ok(()),
		}));
		assert_eq!(DelayTasks::delayed_tasks(2), None);
		assert_eq!(DelayTasks::delayed_tasks(3), None);
		assert_eq!(DelayTasks::delayed_task_queue(101, 2), None);
		assert_eq!(DelayTasks::delayed_task_queue(101, 3), None);
	});
}
