//! Mocks for the delay tasks module.

#![cfg(test)]

use super::*;
use frame_support::{
	construct_runtime, derive_impl, parameter_types,
	traits::{ConstU128, EqualPrivilegeOnly, Everything},
};
use frame_system::EnsureRoot;
use orml_traits::{define_combined_task_and_bind_delay_hooks, parameter_type_with_key, task::TaskResult};
use serde::{Deserialize, Serialize};
use sp_runtime::{traits::IdentityLookup, AccountId32, BuildStorage, DispatchError};
use sp_std::cell::RefCell;

use crate as delay_tasks;

pub type AccountId = AccountId32;
pub type Amount = i128;
pub type Balance = u128;
pub type ReserveIdentifier = [u8; 8];

pub const ALICE: AccountId32 = AccountId32::new([0u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([1u8; 32]);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
	type Nonce = u64;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	type Balance = Balance;
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = [u8; 8];
	type MaxFreezes = ();
}

impl pallet_preimage::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = ();
	type ManagerOrigin = EnsureRoot<AccountId>;
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
	type ScheduleOrigin = EnsureRoot<AccountId>;
	type MaxScheduledPerBlock = ConstU32<10>;
	type WeightInfo = ();
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type Preimages = Preimage;
}

#[derive(
	Encode,
	Decode,
	Eq,
	PartialEq,
	Copy,
	Clone,
	RuntimeDebug,
	PartialOrd,
	Ord,
	parity_scale_codec::MaxEncodedLen,
	TypeInfo,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CurrencyId {
	/// Relay chain token.
	R,
	/// Parachain A token.
	A,
	/// Parachain B token.
	B,
}

pub struct CurrencyIdConvert;
impl Convert<CurrencyId, Option<Location>> for CurrencyIdConvert {
	fn convert(id: CurrencyId) -> Option<Location> {
		match id {
			CurrencyId::R => Some(Parent.into()),
			CurrencyId::A => Some(
				(
					Parent,
					Parachain(1),
					Junction::from(BoundedVec::try_from(b"A".to_vec()).unwrap()),
				)
					.into(),
			),
			CurrencyId::B => Some(
				(
					Parent,
					Parachain(2),
					Junction::from(BoundedVec::try_from(b"B".to_vec()).unwrap()),
				)
					.into(),
			),
		}
	}
}
impl Convert<Location, Option<CurrencyId>> for CurrencyIdConvert {
	fn convert(l: Location) -> Option<CurrencyId> {
		let mut a: Vec<u8> = "A".into();
		a.resize(32, 0);
		let mut b: Vec<u8> = "B".into();
		b.resize(32, 0);

		if l == Location::parent() {
			return Some(CurrencyId::R);
		}
		match l.unpack() {
			(parents, interior) if parents == 1 => match interior {
				[Parachain(1), GeneralKey { data, .. }] if data.to_vec() == a => Some(CurrencyId::A),
				[Parachain(2), GeneralKey { data, .. }] if data.to_vec() == b => Some(CurrencyId::B),
				_ => None,
			},
			(parents, interior) if parents == 0 => match interior {
				[GeneralKey { data, .. }] if data.to_vec() == a => Some(CurrencyId::A),
				[GeneralKey { data, .. }] if data.to_vec() == b => Some(CurrencyId::B),
				_ => None,
			},
			_ => None,
		}
	}
}
impl Convert<Asset, Option<CurrencyId>> for CurrencyIdConvert {
	fn convert(a: Asset) -> Option<CurrencyId> {
		if let Asset {
			fun: Fungible(_),
			id: AssetId(id),
		} = a
		{
			Self::convert(id)
		} else {
			Option::None
		}
	}
}

parameter_type_with_key! {
	pub ExistentialDeposits: |_currency_id: CurrencyId| -> Balance {
		Default::default()
	};
}

impl orml_tokens::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type Amount = Amount;
	type CurrencyId = CurrencyId;
	type WeightInfo = ();
	type ExistentialDeposits = ExistentialDeposits;
	type CurrencyHooks = ();
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = ReserveIdentifier;
	type DustRemovalWhitelist = Everything;
}

thread_local! {
	pub static DISPATCH_SUCCEEDED: RefCell<u32> = RefCell::new(0);
	pub static DISPATCH_FAILED: RefCell<u32> = RefCell::new(0);
	pub static PRE_DELAY_SUCCEEDED: RefCell<u32> = RefCell::new(0);
	pub static PRE_DELAYED_EXECUTE_SUCCEEDED: RefCell<u32> = RefCell::new(0);
	pub static PRE_CANCEL_SUCCEEDED: RefCell<u32> = RefCell::new(0);
}

#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
pub struct SuccessTask;
impl DispatchableTask for SuccessTask {
	fn dispatch(self, _weight: Weight) -> TaskResult {
		DISPATCH_SUCCEEDED.with(|v| *v.borrow_mut() += 1);

		TaskResult {
			result: Ok(()),
			used_weight: Weight::zero(),
			finished: true,
		}
	}
}
pub struct SuccessTaskHook;
impl DelayTaskHooks<SuccessTask> for SuccessTaskHook {
	fn pre_delay(_: &SuccessTask) -> DispatchResult {
		PRE_DELAY_SUCCEEDED.with(|v| *v.borrow_mut() += 1);
		Ok(())
	}
	fn pre_delayed_execute(_: &SuccessTask) -> DispatchResult {
		PRE_DELAYED_EXECUTE_SUCCEEDED.with(|v| *v.borrow_mut() += 1);
		Ok(())
	}
	fn pre_cancel(_: &SuccessTask) -> DispatchResult {
		PRE_CANCEL_SUCCEEDED.with(|v| *v.borrow_mut() += 1);
		Ok(())
	}
}

#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
pub struct FailDispatchTask;
impl DispatchableTask for FailDispatchTask {
	fn dispatch(self, _weight: Weight) -> TaskResult {
		DISPATCH_FAILED.with(|v| *v.borrow_mut() += 1);

		TaskResult {
			result: Err(DispatchError::Other("dispatch failed")),
			used_weight: Weight::zero(),
			finished: true,
		}
	}
}
pub struct FailDispatchTaskHook;
impl DelayTaskHooks<FailDispatchTask> for FailDispatchTaskHook {
	fn pre_delay(_: &FailDispatchTask) -> DispatchResult {
		PRE_DELAY_SUCCEEDED.with(|v| *v.borrow_mut() += 1);
		Ok(())
	}
	fn pre_delayed_execute(_: &FailDispatchTask) -> DispatchResult {
		PRE_DELAYED_EXECUTE_SUCCEEDED.with(|v| *v.borrow_mut() += 1);
		Ok(())
	}
	fn pre_cancel(_: &FailDispatchTask) -> DispatchResult {
		PRE_CANCEL_SUCCEEDED.with(|v| *v.borrow_mut() += 1);
		Ok(())
	}
}

#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
pub struct FailPreDelayTask;
impl DispatchableTask for FailPreDelayTask {
	fn dispatch(self, _weight: Weight) -> TaskResult {
		unimplemented!()
	}
}
pub struct FailPreDelayTaskHook;
impl DelayTaskHooks<FailPreDelayTask> for FailPreDelayTaskHook {
	fn pre_delay(_: &FailPreDelayTask) -> DispatchResult {
		Err(DispatchError::Other("pre_delay failed"))
	}
	fn pre_delayed_execute(_: &FailPreDelayTask) -> DispatchResult {
		unimplemented!()
	}
	fn pre_cancel(_: &FailPreDelayTask) -> DispatchResult {
		unimplemented!()
	}
}

#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
pub struct FailPreDelayedExecuteTask;
impl DispatchableTask for FailPreDelayedExecuteTask {
	fn dispatch(self, _weight: Weight) -> TaskResult {
		unimplemented!()
	}
}
pub struct FailPreDelayedExecuteTaskHook;
impl DelayTaskHooks<FailPreDelayedExecuteTask> for FailPreDelayedExecuteTaskHook {
	fn pre_delay(_: &FailPreDelayedExecuteTask) -> DispatchResult {
		PRE_DELAY_SUCCEEDED.with(|v| *v.borrow_mut() += 1);
		Ok(())
	}
	fn pre_delayed_execute(_: &FailPreDelayedExecuteTask) -> DispatchResult {
		Err(DispatchError::Other("pre_delayed_execute failed"))
	}
	fn pre_cancel(_: &FailPreDelayedExecuteTask) -> DispatchResult {
		PRE_CANCEL_SUCCEEDED.with(|v| *v.borrow_mut() += 1);
		Ok(())
	}
}

#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
pub struct FailPreCancelTask;
impl DispatchableTask for FailPreCancelTask {
	fn dispatch(self, _weight: Weight) -> TaskResult {
		DISPATCH_SUCCEEDED.with(|v| *v.borrow_mut() += 1);

		TaskResult {
			result: Ok(()),
			used_weight: Weight::zero(),
			finished: true,
		}
	}
}
pub struct FailPreCancelTaskHook;
impl DelayTaskHooks<FailPreCancelTask> for FailPreCancelTaskHook {
	fn pre_delay(_: &FailPreCancelTask) -> DispatchResult {
		PRE_DELAY_SUCCEEDED.with(|v| *v.borrow_mut() += 1);
		Ok(())
	}
	fn pre_delayed_execute(_: &FailPreCancelTask) -> DispatchResult {
		PRE_DELAYED_EXECUTE_SUCCEEDED.with(|v| *v.borrow_mut() += 1);
		Ok(())
	}
	fn pre_cancel(_: &FailPreCancelTask) -> DispatchResult {
		Err(DispatchError::Other("pre_cancel failed"))
	}
}

define_combined_task_and_bind_delay_hooks! {
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	pub enum MockTaskType {
		Success(SuccessTask, SuccessTaskHook),
		FailDispatch(FailDispatchTask, FailDispatchTaskHook),
		FailPreDelay(FailPreDelayTask, FailPreDelayTaskHook),
		FailPreDelayedExecute(FailPreDelayedExecuteTask, FailPreDelayedExecuteTaskHook),
		FailPreCancel(FailPreCancelTask, FailPreCancelTaskHook),
	}
}

parameter_types! {
	pub ReserveId: ReserveIdentifier = [1u8;8];
}

impl delay_tasks::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type DelayOrigin = EnsureDelayed;
	type GovernanceOrigin = EnsureRoot<AccountId>;
	type Task = MockTaskType;
	type Scheduler = Scheduler;
	type DelayTaskHooks = MockTaskType;
	type CurrencyIdConvert = CurrencyIdConvert;
	type Currency = Tokens;
	type ReserveId = ReserveId;
}

type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime {
		System: frame_system,
		DelayTasks: delay_tasks,
		Scheduler: pallet_scheduler,
		Preimage: pallet_preimage,
		Tokens: orml_tokens,
		Balances: pallet_balances,
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
