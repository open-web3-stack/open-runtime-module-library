//! Mocks for the authority module.

#![cfg(test)]

use super::*;
use codec::{Decode, Encode};
use frame_support::{
	parameter_types,
	traits::{ConstU64, EqualPrivilegeOnly, Everything},
	weights::Weight,
};
use frame_system::{ensure_root, ensure_signed, EnsureRoot};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BadOrigin, IdentityLookup},
	Perbill,
};

pub use crate as authority;

pub type AccountId = u128;
pub type BlockNumber = u64;

parameter_types! {
	pub BlockWeights: frame_system::limits::BlockWeights =
			frame_system::limits::BlockWeights::simple_max(Weight::from_ref_time(2_000_000_000_000).set_proof_size(u64::MAX));
}

impl frame_system::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type BlockWeights = BlockWeights;
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = Everything;
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

impl pallet_preimage::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = ();
	type ManagerOrigin = EnsureRoot<u128>;
	type BaseDeposit = ();
	type ByteDeposit = ();
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * BlockWeights::get().max_block;
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

impl pallet_root_testing::Config for Runtime {}

#[derive(Clone, Encode, Decode, Eq, PartialEq, Ord, PartialOrd, Debug, TypeInfo)]
pub enum MockAsOriginId {
	Root,
	Account1,
	Account2,
}

pub struct AuthorityConfigImpl;

impl AuthorityConfig<RuntimeOrigin, OriginCaller, BlockNumber> for AuthorityConfigImpl {
	fn check_schedule_dispatch(origin: RuntimeOrigin, _priority: Priority) -> DispatchResult {
		let origin: Result<frame_system::RawOrigin<u128>, _> = origin.into();
		match origin {
			Ok(frame_system::RawOrigin::Root)
			| Ok(frame_system::RawOrigin::Signed(1))
			| Ok(frame_system::RawOrigin::Signed(2)) => Ok(()),
			_ => Err(BadOrigin.into()),
		}
	}
	fn check_fast_track_schedule(
		origin: RuntimeOrigin,
		_initial_origin: &OriginCaller,
		_new_delay: BlockNumber,
	) -> DispatchResult {
		ensure_root(origin)?;
		Ok(())
	}
	fn check_delay_schedule(origin: RuntimeOrigin, initial_origin: &OriginCaller) -> DispatchResult {
		ensure_root(origin.clone()).or_else(|_| {
			if origin.caller() == initial_origin {
				Ok(())
			} else {
				Err(BadOrigin.into())
			}
		})
	}
	fn check_cancel_schedule(origin: RuntimeOrigin, initial_origin: &OriginCaller) -> DispatchResult {
		ensure_root(origin.clone()).or_else(|_| {
			if origin.caller() == initial_origin {
				Ok(())
			} else {
				Err(BadOrigin.into())
			}
		})
	}
}

impl AsOriginId<RuntimeOrigin, OriginCaller> for MockAsOriginId {
	fn into_origin(self) -> OriginCaller {
		match self {
			MockAsOriginId::Root => RuntimeOrigin::root().caller().clone(),
			MockAsOriginId::Account1 => RuntimeOrigin::signed(1).caller().clone(),
			MockAsOriginId::Account2 => RuntimeOrigin::signed(2).caller().clone(),
		}
	}
	fn check_dispatch_from(&self, origin: RuntimeOrigin) -> DispatchResult {
		ensure_root(origin.clone()).or_else(|_| {
			if let OriginCaller::Authority(ref sign) = origin.caller() {
				if sign.origin == Box::new(RuntimeOrigin::root().caller().clone()) {
					return Ok(());
				} else {
					return Err(BadOrigin.into());
				}
			}

			let ok = match self {
				MockAsOriginId::Root => false,
				MockAsOriginId::Account1 => ensure_signed(origin)? == 1,
				MockAsOriginId::Account2 => ensure_signed(origin)? == 2,
			};
			if ok {
				Ok(())
			} else {
				Err(BadOrigin.into())
			}
		})
	}
}

impl Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type Scheduler = Scheduler;
	type RuntimeCall = RuntimeCall;
	type AsOriginId = MockAsOriginId;
	type AuthorityConfig = AuthorityConfigImpl;
	type WeightInfo = ();
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Runtime>;
type Block = frame_system::mocking::MockBlock<Runtime>;

frame_support::construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Pallet, Call, Config, Event<T>},
		Authority: authority::{Pallet, Call, Origin<T>, Event<T>},
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>},
		Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>},
		RootTesting: pallet_root_testing::{Pallet, Call},
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
		let t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
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
