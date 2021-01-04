#![cfg(test)]

use super::*;

use frame_support::{impl_outer_dispatch, impl_outer_event, impl_outer_origin, parameter_types};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

use std::cell::RefCell;

mod oracle {
	pub use super::super::*;
}

impl_outer_event! {
	pub enum TestEvent for Test {
		frame_system<T>,
		oracle<T>,
	}
}

impl_outer_origin! {
	pub enum Origin for Test {}
}

impl_outer_dispatch! {
	pub enum Call for Test where origin: Origin {
		oracle::ModuleOracle,
	}
}

pub type AccountId = u128;
type Key = u32;
type Value = u32;

// For testing the module, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of modules we want to use.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Test;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
}
impl frame_system::Config for Test {
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = TestEvent;
	type BlockHashCount = BlockHashCount;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = ();
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
}
pub type System = frame_system::Module<Test>;

thread_local! {
	static TIME: RefCell<u32> = RefCell::new(0);
}

pub struct Timestamp;
impl Time for Timestamp {
	type Moment = u32;

	fn now() -> Self::Moment {
		TIME.with(|v| *v.borrow())
	}
}

impl Timestamp {
	pub fn set_timestamp(val: u32) {
		TIME.with(|v| *v.borrow_mut() = val);
	}
}

parameter_types! {
	pub const MinimumCount: u32 = 3;
	pub const ExpiresIn: u32 = 600;
	pub const RootOperatorAccountId: AccountId = 4;
}

impl Config for Test {
	type Event = TestEvent;
	type OnNewData = ();
	type CombineData = DefaultCombineData<Self, MinimumCount, ExpiresIn>;
	type Time = Timestamp;
	type OracleKey = Key;
	type OracleValue = Value;
	type RootOperatorAccountId = RootOperatorAccountId;
	type WeightInfo = ();
}

pub type ModuleOracle = Module<Test>;
// This function basically just builds a genesis storage key/value store
// according to our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	let _ = GenesisConfig::<Test> {
		members: vec![1, 2, 3].into(),
		phantom: Default::default(),
	}
	.assimilate_storage(&mut storage);

	let mut t: sp_io::TestExternalities = storage.into();

	t.execute_with(|| {
		Timestamp::set_timestamp(12345);
	});

	t
}
