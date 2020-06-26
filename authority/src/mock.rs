//! Mocks for the authority module.

#![cfg(test)]

use frame_support::{ord_parameter_types, parameter_types};
use frame_system::EnsureSignedBy;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{Block as BlockT, IdentityLookup},
	Perbill,
};

use super::*;
use crate as authority;

pub type AccountId = u128;
pub type BlockNumber = u64;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const MaximumBlockWeight: u32 = 1024;
	pub const MaximumBlockLength: u32 = 2 * 1024;
	pub const AvailableBlockRatio: Perbill = Perbill::one();
}

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
}

pub struct MockScheduler;
impl Scheduler<BlockNumber> for MockScheduler {
	type Origin = Origin;
	type Call = Call;

	fn schedule(_: Self::Origin, _: Self::Call, _: DelayedDispatchTime<BlockNumber>) -> DispatchId {
		Default::default()
	}

	fn cancel(_: DispatchId) {}
}

ord_parameter_types! {
	pub const One: AccountId = 1;
	pub const Two: AccountId = 2;
}

parameter_types! {
	pub const MinimumDelay: BlockNumber = 10;
}

impl Trait for Runtime {
	type Origin = Origin;
	type Call = Call;
	type RootDispatchOrigin = EnsureSignedBy<One, AccountId>;
	type DelayedRootDispatchOrigin = EnsureSignedBy<One, AccountId>;
	type DelayedDispatchOrigin = EnsureSignedBy<One, AccountId>;
	type VetoOrigin = EnsureSignedBy<One, AccountId>;
	type InstantDispatchOrigin = EnsureSignedBy<Two, AccountId>;
	type Scheduler = MockScheduler;
	type MinimumDelay = MinimumDelay;
}

pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<u32, u64, Call, ()>;

frame_support::construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		System: frame_system::{Module, Call, Event<T>},
		Authority: authority::{Module, Call, Origin<T>},
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
