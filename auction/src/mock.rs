//! Mocks for the auction module.

#![cfg(test)]

use super::*;
use frame_support::{construct_runtime, derive_impl};
use orml_traits::OnNewBidResult;
use sp_runtime::{traits::IdentityLookup, BuildStorage};

use crate as auction;

pub type AccountId = u128;
pub type Balance = u64;
pub type BlockNumber = u64;
pub type AuctionId = u64;

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Runtime {
	type Nonce = u64;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
}

pub struct Handler;

impl AuctionHandler<AccountId, Balance, BlockNumber, AuctionId> for Handler {
	fn on_new_bid(
		now: BlockNumber,
		_id: AuctionId,
		new_bid: (AccountId, Balance),
		last_bid: Option<(AccountId, Balance)>,
	) -> OnNewBidResult<BlockNumber> {
		if last_bid.is_none() || last_bid.unwrap().0 != new_bid.0 {
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
	type Balance = Balance;
	type AuctionId = AuctionId;
	type Handler = Handler;
	type WeightInfo = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = BaseBenchmarkHelper<Runtime>;
}

type Block = frame_system::mocking::MockBlock<Runtime>;

construct_runtime!(
	pub enum Runtime {
		System: frame_system,
		AuctionModule: auction,
	}
);

pub const ALICE: AccountId = 1;
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
