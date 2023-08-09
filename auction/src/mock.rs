//! Mocks for the auction module.

#![cfg(test)]

use super::*;
use frame_support::{
	construct_runtime,
	traits::{ConstU64, Everything},
};
use orml_traits::OnNewBidResult;
use sp_core::H256;
use sp_runtime::{traits::IdentityLookup, BuildStorage};

use crate as auction;

pub type AccountId = u128;
pub type Balance = u64;
pub type BlockNumber = u64;
pub type AuctionId = u64;

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
	type DbWeight = ();
	type BaseCallFilter = Everything;
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

pub struct Handler;

impl AuctionHandler<AccountId, Balance, BlockNumber, AuctionId> for Handler {
	fn on_new_bid(
		now: BlockNumber,
		_id: AuctionId,
		new_bid: (AccountId, Balance),
		_last_bid: Option<(AccountId, Balance)>,
	) -> OnNewBidResult<BlockNumber> {
		if new_bid.0 == ALICE {
			OnNewBidResult {
				accept_bid: true,
				auction_end_change: Change::NewValue(Some(now + BID_EXTEND_BLOCK)),
			}
		} else {
			OnNewBidResult {
				accept_bid: false,
				auction_end_change: Change::NoChange,
			}
		}
	}

	fn on_auction_ended(_id: AuctionId, _winner: Option<(AccountId, Balance)>) {}
}

impl Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AuctionId = AuctionId;
	type Handler = Handler;
	type WeightInfo = ();
}

type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime {
		System: frame_system,
		AuctionModule: auction,
	}
);

pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;
pub const BID_EXTEND_BLOCK: BlockNumber = 10;

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
