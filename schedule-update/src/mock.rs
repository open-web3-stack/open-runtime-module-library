//! Mocks for the schedule-update module.

#![cfg(test)]

use frame_support::{impl_outer_dispatch, impl_outer_event, impl_outer_origin, parameter_types};
use frame_system::{self as system, ensure_signed};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};

use super::*;

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

mod schedule_update {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum TestEvent for Runtime {
		frame_system<T>,
		schedule_update<T>,
		pallet_balances<T>,
	}
}

impl_outer_dispatch! {
	pub enum Call for Runtime where origin: Origin {
		pallet_balances::Balances,
	}
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Runtime;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}

pub type AccountId = u64;
pub type BlockNumber = u64;

impl frame_system::Trait for Runtime {
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Call = Call;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = TestEvent;
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type ModuleToIndex = ();
	type AccountData = pallet_balances::AccountData<u128>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
}
pub type System = frame_system::Module<Runtime>;

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Trait for Runtime {
	type Balance = u128;
	type DustRemoval = ();
	type Event = TestEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
}

pub const ALICE: AccountId = 1u64;
pub const BOB: AccountId = 2u64;

// A mock schedule origin where only `ALICE` has permission.
pub struct MockScheduleOrigin;

impl EnsureOrigin<Origin> for MockScheduleOrigin {
	type Success = ();

	fn try_origin(o: Origin) -> Result<Self::Success, Origin> {
		let who = ensure_signed(o.clone()).map_err(|_| o.clone())?;
		if who == ALICE {
			Ok(())
		} else {
			Err(o)
		}
	}
}

parameter_types! {
	pub const MaxScheduleDispatchWeight: Weight = 150_000_000;
}

impl Trait for Runtime {
	type Event = TestEvent;
	type DispatchOrigin = MockScheduleOrigin;
	type Call = Call;
	type MaxScheduleDispatchWeight = MaxScheduleDispatchWeight;
}
pub type ScheduleUpdateModule = Module<Runtime>;

pub type Balances = pallet_balances::Module<Runtime>;

pub type BalancesCall = pallet_balances::Call<Runtime>;

pub struct ExtBuilder;

impl Default for ExtBuilder {
	fn default() -> Self {
		ExtBuilder
	}
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		pallet_balances::GenesisConfig::<Runtime> {
			balances: vec![(1, 100), (2, 100), (3, 100), (4, 100), (5, 100)],
		}
		.assimilate_storage(&mut t)
		.unwrap();

		t.into()
	}
}
