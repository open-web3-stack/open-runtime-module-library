//! Mocks for the delay tasks module.

#![cfg(test)]

use super::*;
use frame_support::{construct_runtime, derive_impl};
use frame_system::EnsureRoot;
use orml_traits::define_combined_delayed_task;
use sp_runtime::{traits::IdentityLookup, BuildStorage, DispatchError};
use sp_std::cell::RefCell;

use crate as delay_tasks;

pub type AccountId = u128;

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
	type Nonce = u64;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
}

thread_local! {
	pub static PRE_DELAYED: RefCell<bool> = RefCell::new(false);
	pub static PRE_DELAYED_EXECUTED: RefCell<bool> = RefCell::new(false);
	pub static EXECUTED: RefCell<bool> = RefCell::new(false);
	pub static ON_CANCEL: RefCell<bool> = RefCell::new(false);
}

pub(crate) fn reset_delay_process_records() {
	PRE_DELAYED.with(|v| *v.borrow_mut() = false);
	PRE_DELAYED_EXECUTED.with(|v| *v.borrow_mut() = false);
	EXECUTED.with(|v| *v.borrow_mut() = false);
	ON_CANCEL.with(|v| *v.borrow_mut() = false);
}

define_combined_delayed_task! {
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	pub enum MockDelayedTaskType {
		Success(SuccessTask),
		FailedPreDelay(FailedPreDelayTask),
		FailedPreDelayedExecute(FailedPreDelayedExecuteTask),
		FailedDelayedExecute(FailedDelayedExecuteTask),
		FailedOnCancel(FailedOnCancelTask),
	}
}

#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
pub struct SuccessTask;
impl DelayedTask for SuccessTask {
	fn pre_delay(&self) -> DispatchResult {
		PRE_DELAYED.with(|v| *v.borrow_mut() = true);
		Ok(())
	}

	fn pre_delayed_execute(&self) -> DispatchResult {
		PRE_DELAYED_EXECUTED.with(|v| *v.borrow_mut() = true);
		Ok(())
	}

	fn delayed_execute(&self) -> DispatchResult {
		EXECUTED.with(|v| *v.borrow_mut() = true);
		Ok(())
	}

	fn on_cancel(&self) -> DispatchResult {
		ON_CANCEL.with(|v| *v.borrow_mut() = true);
		Ok(())
	}
}

#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
pub struct FailedPreDelayTask;
impl DelayedTask for FailedPreDelayTask {
	fn pre_delay(&self) -> DispatchResult {
		Err(DispatchError::Other("pre_delay failed"))
	}

	fn pre_delayed_execute(&self) -> DispatchResult {
		unimplemented!()
	}

	fn delayed_execute(&self) -> DispatchResult {
		unimplemented!()
	}

	fn on_cancel(&self) -> DispatchResult {
		unimplemented!()
	}
}

#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
pub struct FailedPreDelayedExecuteTask;
impl DelayedTask for FailedPreDelayedExecuteTask {
	fn pre_delay(&self) -> DispatchResult {
		PRE_DELAYED.with(|v| *v.borrow_mut() = true);
		Ok(())
	}

	fn pre_delayed_execute(&self) -> DispatchResult {
		Err(DispatchError::Other("pre_delayed_execute failed"))
	}

	fn delayed_execute(&self) -> DispatchResult {
		EXECUTED.with(|v| *v.borrow_mut() = true);
		Ok(())
	}

	fn on_cancel(&self) -> DispatchResult {
		ON_CANCEL.with(|v| *v.borrow_mut() = true);
		Ok(())
	}
}

#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
pub struct FailedDelayedExecuteTask;
impl DelayedTask for FailedDelayedExecuteTask {
	fn pre_delay(&self) -> DispatchResult {
		PRE_DELAYED.with(|v| *v.borrow_mut() = true);
		Ok(())
	}

	fn pre_delayed_execute(&self) -> DispatchResult {
		PRE_DELAYED_EXECUTED.with(|v| *v.borrow_mut() = true);
		Ok(())
	}

	fn delayed_execute(&self) -> DispatchResult {
		Err(DispatchError::Other("delayed_execute failed"))
	}

	fn on_cancel(&self) -> DispatchResult {
		ON_CANCEL.with(|v| *v.borrow_mut() = true);
		Ok(())
	}
}

#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
pub struct FailedOnCancelTask;
impl DelayedTask for FailedOnCancelTask {
	fn pre_delay(&self) -> DispatchResult {
		PRE_DELAYED.with(|v| *v.borrow_mut() = true);
		Ok(())
	}

	fn pre_delayed_execute(&self) -> DispatchResult {
		PRE_DELAYED_EXECUTED.with(|v| *v.borrow_mut() = true);
		Ok(())
	}

	fn delayed_execute(&self) -> DispatchResult {
		EXECUTED.with(|v| *v.borrow_mut() = true);
		Ok(())
	}

	fn on_cancel(&self) -> DispatchResult {
		Err(DispatchError::Other("on_cancel failed"))
	}
}

impl Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type GovernanceOrigin = EnsureRoot<AccountId>;
	type Task = MockDelayedTaskType;
}

type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime {
		System: frame_system,
		DelayTasks: delay_tasks,
	}
);

pub struct ExtBuilder;

impl Default for ExtBuilder {
	fn default() -> Self {
		ExtBuilder
	}
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let t = frame_system::GenesisConfig::<Runtime>::default()
			.build_storage()
			.unwrap();

		t.into()
	}
}
