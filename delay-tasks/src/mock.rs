//! Mocks for the delay tasks module.

#![cfg(test)]

use super::*;
use frame_support::{construct_runtime, derive_impl, parameter_types, traits::EqualPrivilegeOnly};
use frame_system::EnsureRoot;
use orml_traits::define_combined_task;
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

impl pallet_preimage::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = ();
	type ManagerOrigin = EnsureRoot<u128>;
	type Consideration = ();
}

parameter_types! {
	pub BlockWeights: frame_system::limits::BlockWeights =
			frame_system::limits::BlockWeights::simple_max(Weight::from_parts(2_000_000_000_000, 0).set_proof_size(u64::MAX));
	pub MaximumSchedulerWeight: Weight = BlockWeights::get().max_block;
}

impl pallet_scheduler::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<u128>;
	type MaxScheduledPerBlock = ConstU32<10>;
	type WeightInfo = ();
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type Preimages = Preimage;
}

thread_local! {
	pub static SUCCEEDED: RefCell<u32> = RefCell::new(0);
	pub static FAILED: RefCell<u32> = RefCell::new(0);
}

define_combined_task! {
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	pub enum MockTaskType {
		Success(SuccessTask),
		Fail(FailTask),
	}
}

#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
pub struct SuccessTask;
impl DispatchableTask for SuccessTask {
	fn dispatch(self, _weight: Weight) -> TaskResult {
		SUCCEEDED.with(|v| *v.borrow_mut() += 1);

		TaskResult {
			result: Ok(()),
			used_weight: Weight::zero(),
			finished: true,
		}
	}
}

#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
pub struct FailTask;
impl DispatchableTask for FailTask {
	fn dispatch(self, _weight: Weight) -> TaskResult {
		FAILED.with(|v| *v.borrow_mut() += 1);

		TaskResult {
			result: Err(DispatchError::Other("execute failed")),
			used_weight: Weight::zero(),
			finished: true,
		}
	}
}

impl Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type DelayOrigin = EnsureDelayed;
	type GovernanceOrigin = EnsureRoot<AccountId>;
	type Task = MockTaskType;
	type Scheduler = Scheduler;
}

type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime {
		System: frame_system,
		DelayTasks: delay_tasks,
		Scheduler: pallet_scheduler,
		Preimage: pallet_preimage,
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

pub fn run_to_block(n: u64) {
	while System::block_number() < n {
		Scheduler::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		Scheduler::on_initialize(System::block_number());
	}
}
