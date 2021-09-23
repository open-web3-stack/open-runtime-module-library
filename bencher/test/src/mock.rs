#![cfg(any(test, feature = "bench"))]

use frame_support::parameter_types;
use sp_runtime::{
	MultiSignature,
	traits::{BlakeTwo256, IdentityLookup}
};
use sp_core::H256;
use sp_std::prelude::*;

pub type Signature = MultiSignature;
pub type BlockNumber = u64;
pub type AccountId = u32;
pub type Address = sp_runtime::MultiAddress<AccountId, u32>;
pub type Header = sp_runtime::generic::Header<BlockNumber, BlakeTwo256>;

pub type SignedExtra = (
	frame_system::CheckWeight<Runtime>,
);

pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;

pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;

frame_support::construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		TestPallet: crate::pallet_test::{Pallet, Call, Storage},
	}
);

impl frame_system::Config for Runtime {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Call = Call;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
}

parameter_types! {
	pub const LowerBound: u32 = 1;
	pub const UpperBound: u32 = 100;
}

impl crate::pallet_test::Config for Runtime {
	type Event = Event;
	type LowerBound = LowerBound;
	type UpperBound = UpperBound;
}

impl crate::pallet_test::OtherConfig for Runtime {
	type OtherEvent = Event;
}
