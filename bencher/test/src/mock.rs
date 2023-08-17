#![cfg(any(test, feature = "bench"))]

use frame_support::pallet_prelude::ConstU32;
use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage, MultiSignature,
};
use sp_std::prelude::*;

pub type Signature = MultiSignature;
pub type BlockNumber = u64;
pub type AccountId = u32;
pub type Address = sp_runtime::MultiAddress<AccountId, u32>;
pub type Header = sp_runtime::generic::Header<BlockNumber, BlakeTwo256>;

pub type SignedExtra = (frame_system::CheckWeight<Runtime>,);

pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;

pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;

frame_support::construct_runtime!(
	pub enum Runtime {
		System: frame_system,
		Test: crate::pallet,
	}
);

impl frame_system::Config for Runtime {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type Nonce = u64;
	type Hash = H256;
	type RuntimeCall = RuntimeCall;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

impl crate::pallet::Config for Runtime {}

#[cfg(test)]
pub struct ExtBuilder;

#[cfg(test)]
impl Default for ExtBuilder {
	fn default() -> Self {
		Self {}
	}
}

#[cfg(test)]
impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let t = frame_system::GenesisConfig::<Runtime>::default()
			.build_storage()
			.unwrap();

		t.into()
	}
}
