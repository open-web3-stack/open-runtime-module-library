#![cfg(test)]

use super::*;

use primitives::H256;
use sr_primitives::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	weights::Weight,
	Perbill,
};
use support::{impl_outer_origin, parameter_types, traits::Time};

impl_outer_origin! {
	pub enum Origin for Test {}
}

// For testing the module, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of modules we want to use.
#[derive(Clone, Eq, PartialEq)]
pub struct Test;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: Weight = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
}
impl system::Trait for Test {
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type MaximumBlockWeight = MaximumBlockWeight;
	type MaximumBlockLength = MaximumBlockLength;
	type AvailableBlockRatio = AvailableBlockRatio;
	type Version = ();
}

type AccountId = u64;
type Moment = u64;
type Key = u32;
type Value = u32;

static mut TIMESTAMP: u64 = 0;

pub struct MockTime;

impl MockTime {
	pub fn set_time(sec: Moment) {
		unsafe {
			TIMESTAMP = sec;
		}
	}
}

impl Time for MockTime {
	type Moment = Moment;
	fn now() -> Self::Moment {
		unsafe { TIMESTAMP.clone() }
	}
}

pub struct MockOperatorProvider;

impl OperatorProvider<AccountId> for MockOperatorProvider {
	fn can_feed_data(who: &AccountId) -> bool {
		Self::operators().contains(who)
	}

	fn operators() -> Vec<AccountId> {
		vec![1, 2, 3]
	}
}

pub struct MockCombineData;

/// This mock implementation will just return first valid value
impl CombineData<Key, TimestampedValue<Value, Moment>> for MockCombineData {
	fn combine_data(
		_key: &Key,
		values: Vec<TimestampedValue<Value, Moment>>,
		_prev_value: Option<TimestampedValue<Value, Moment>>,
	) -> Option<TimestampedValue<Value, Moment>> {
		if values.len() == 0 {
			return None;
		}
		let value = values[0].value;
		Some(TimestampedValue {
			value,
			timestamp: MockTime::now(),
		})
	}
}

impl Trait for Test {
	type Event = ();
	type OnNewData = ();
	type OperatorProvider = MockOperatorProvider;
	type CombineData = MockCombineData;
	type Time = MockTime;
	type Key = Key;
	type Value = Value;
}
pub type ModuleOracle = Module<Test>;

// This function basically just builds a genesis storage key/value store according to
// our desired mockup.
pub fn new_test_ext() -> runtime_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}
