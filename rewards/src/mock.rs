//! Mocks for the rewards module.

#![cfg(test)]

use super::*;
use frame_support::{impl_outer_origin, parameter_types};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};
use sp_std::cell::RefCell;
use std::collections::HashMap;

pub type AccountId = u128;
pub type Balance = u64;
pub type Share = u64;
pub type PoolId = u32;
pub type BlockNumber = u64;
pub type CurrencyId = u32;

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const CAROL: AccountId = 3;
pub const DOT_POOL: PoolId = 1;
pub const BTC_POOL: PoolId = 2;
pub const XBTC_POOL: PoolId = 3;

impl_outer_origin! {
	pub enum Origin for Runtime {}
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

impl frame_system::Trait for Runtime {
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Call = ();
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
	type PalletInfo = ();
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
	pub static RECEIVED_PAYOUT: RefCell<HashMap<(PoolId, AccountId), Balance>> = RefCell::new(HashMap::new());
}

pub struct Handler;
impl RewardHandler<AccountId, BlockNumber> for Handler {
	type Share = Share;
	type Balance = Balance;
	type PoolId = PoolId;
	type CurrencyId = CurrencyId;

	fn accumulate_reward(
		now: BlockNumber,
		mut callback: impl FnMut(Self::PoolId, Self::Balance),
	) -> Vec<(Self::CurrencyId, Self::Balance)> {
		if now % 2 == 0 {
			let mut total_accumulated_rewards = 0;
			let valid_pool_ids = vec![DOT_POOL, BTC_POOL];

			for (pool, _) in Pools::<Runtime>::iter() {
				if valid_pool_ids.contains(&pool) {
					let rewards: Balance = 100;
					callback(pool, rewards);
					total_accumulated_rewards += rewards;
				}
			}

			vec![(1, total_accumulated_rewards)]
		} else {
			vec![]
		}
	}

	fn payout(who: &AccountId, pool: Self::PoolId, amount: Self::Balance) {
		RECEIVED_PAYOUT.with(|v| {
			let mut old_map = v.borrow().clone();
			if let Some(before) = old_map.get_mut(&(pool, *who)) {
				*before += amount;
			} else {
				old_map.insert((pool, *who), amount);
			};

			*v.borrow_mut() = old_map;
		});
	}
}

impl Trait for Runtime {
	type Share = Share;
	type Balance = Balance;
	type PoolId = PoolId;
	type Handler = Handler;
	type WeightInfo = ();
}
pub type RewardsModule = Module<Runtime>;

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
