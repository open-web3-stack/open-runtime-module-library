//! Mocks for the rewards module.

#![cfg(test)]

use super::*;
use frame_support::{
	construct_runtime,
	traits::{ConstU64, Everything},
	weights::constants::RocksDbWeight,
};
use sp_core::H256;
use sp_runtime::{traits::IdentityLookup, BuildStorage};
use sp_std::cell::RefCell;
use std::collections::HashMap;

use crate as rewards;

pub type AccountId = u128;
pub type Balance = u64;
pub type Share = u64;
pub type PoolId = u32;
pub type CurrencyId = u32;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CAROL: AccountId = 3;
pub const DOT_POOL: PoolId = 1;
pub const NATIVE_COIN: CurrencyId = 0;
pub const STABLE_COIN: CurrencyId = 1;

impl frame_system::Config for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Nonce = u64;
	type RuntimeCall = RuntimeCall;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = RocksDbWeight;
	type BaseCallFilter = Everything;
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

thread_local! {
	pub static RECEIVED_PAYOUT: RefCell<HashMap<(PoolId, AccountId, CurrencyId), Balance>> = RefCell::new(HashMap::new());
}

pub struct Handler;
impl RewardHandler<AccountId, CurrencyId> for Handler {
	type Balance = Balance;
	type PoolId = PoolId;

	fn payout(who: &AccountId, pool: &Self::PoolId, currency_id: CurrencyId, amount: Self::Balance) {
		RECEIVED_PAYOUT.with(|v| {
			let mut old_map = v.borrow().clone();
			if let Some(before) = old_map.get_mut(&(*pool, *who, currency_id)) {
				*before += amount;
			} else {
				old_map.insert((*pool, *who, currency_id), amount);
			};

			*v.borrow_mut() = old_map;
		});
	}
}

impl Config for Runtime {
	type Share = Share;
	type Balance = Balance;
	type PoolId = PoolId;
	type CurrencyId = CurrencyId;
	type Handler = Handler;
}

type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime {
		System: frame_system,
		RewardsModule: rewards,
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
