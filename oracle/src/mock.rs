#![cfg(test)]

use super::*;

use frame_support::{impl_outer_dispatch, impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_runtime::{
	testing::{Header, UintAuthorityId},
	traits::{BlakeTwo256, IdentityLookup},
	Perbill,
};

use std::cell::RefCell;

impl_outer_origin! {
	pub enum Origin for Test {}
}

impl_outer_dispatch! {
	pub enum Call for Test where origin: Origin {
		oracle::ModuleOracle,
	}
}

pub type OracleCall = super::Call<Test, Instance1>;
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
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}
impl frame_system::Trait for Test {
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
	type ModuleToIndex = ();
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BlockExecutionWeight = ();
	type ExtrinsicBaseWeight = ();
	type MaximumExtrinsicWeight = ();
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
}

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
	pub const UnsignedPriority: TransactionPriority = 32u64;
}

impl Trait<Instance1> for Test {
	type Event = ();
	type OnNewData = ();
	type CombineData = DefaultCombineData<Self, Instance1, MinimumCount, ExpiresIn>;
	type Time = Timestamp;
	type OracleKey = Key;
	type OracleValue = Value;
	type UnsignedPriority = UnsignedPriority;
	type AuthorityId = UintAuthorityId;
}

pub type ModuleOracle = Module<Test, Instance1>;
// This function basically just builds a genesis storage key/value store
// according to our desired mockup.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	let _ = GenesisConfig::<Test, Instance1> {
		members: vec![1, 2, 3].into(),
		session_keys: vec![(1, 10.into()), (2, 20.into()), (3, 30.into())],
	}
	.assimilate_storage(&mut storage);

	let mut t: sp_io::TestExternalities = storage.into();

	t.execute_with(|| {
		Timestamp::set_timestamp(12345);
	});

	t
}
