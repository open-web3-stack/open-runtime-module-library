//! Mocks for the gradually-update module.

#![cfg(test)]

use super::*;
use frame_support::{
	construct_runtime, derive_impl,
	traits::{ConstU32, ConstU64},
};
use sp_runtime::{traits::IdentityLookup, BuildStorage};

use crate as gradually_update;

pub type AccountId = u128;

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
	type Nonce = u64;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
}

impl Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type UpdateFrequency = ConstU64<10>;
	type DispatchOrigin = frame_system::EnsureRoot<AccountId>;
	type WeightInfo = ();
	type MaxGraduallyUpdate = ConstU32<3>;
	type MaxStorageKeyBytes = ConstU32<100_000>;
	type MaxStorageValueBytes = ConstU32<100_000>;
}

type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime {
		System: frame_system,
		GraduallyUpdateModule: gradually_update,
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
