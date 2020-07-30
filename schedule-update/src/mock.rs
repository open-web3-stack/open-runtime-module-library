//! Mocks for the schedule-update module.

#![cfg(test)]

use frame_support::{impl_outer_dispatch, impl_outer_event, impl_outer_origin, parameter_types};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};

use super::*;

mod logger {
	use super::*;
	use frame_system::ensure_root;
	use std::cell::RefCell;

	thread_local! {
		static LOG: RefCell<Vec<u32>> = RefCell::new(Vec::new());
	}
	pub trait Trait: frame_system::Trait {
		type Event: From<Event> + Into<<Self as frame_system::Trait>::Event>;
	}
	decl_storage! {
		trait Store for Module<T: Trait> as Logger {
		}
	}
	decl_event! {
		pub enum Event {
			Logged(u32, Weight),
		}
	}
	decl_module! {
		pub struct Module<T: Trait> for enum Call where origin: <T as frame_system::Trait>::Origin {
			fn deposit_event() = default;

			#[weight = *weight]
			fn log(origin, i: u32, weight: Weight) {
				ensure_root(origin)?;
				Self::deposit_event(Event::Logged(i, weight));
				LOG.with(|log| {
					log.borrow_mut().push(i);
				})
			}
		}
	}
}

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
		logger,
	}
}

impl_outer_dispatch! {
	pub enum Call for Runtime where origin: Origin {
		system::System,
		pallet_balances::Balances,
		logger::Logger,
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

pub type AccountId = u128;
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
	type MaximumExtrinsicWeight = ();
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
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
	type WeightInfo = ();
}
pub type Balances = pallet_balances::Module<Runtime>;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;

impl logger::Trait for Runtime {
	type Event = TestEvent;
}
type Logger = logger::Module<Runtime>;

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

pub type BalancesCall = pallet_balances::Call<Runtime>;
pub type LoggerCall = logger::Call<Runtime>;

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
