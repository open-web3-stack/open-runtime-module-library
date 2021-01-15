//! Mocks for the auction module.

#![cfg(test)]

use super::*;
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types};
use orml_traits::OnNewBidResult;
use orml_traits::{AuctionHandler, Change};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup};

impl_outer_origin! {
	pub enum Origin for Runtime {}
}

mod auction {
	pub use crate::Event;
}

impl_outer_event! {
	pub enum TestEvent for Runtime {
		frame_system<T>,
		auction<T>,
	}
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Runtime;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
}

pub type AccountId = u128;
pub type Balance = u64;
pub type BlockNumber = u64;
pub type AuctionId = u64;

impl frame_system::Config for Runtime {
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = BlockNumber;
	type Call = ();
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = TestEvent;
	type BlockHashCount = BlockHashCount;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = ();
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
}
pub type System = frame_system::Module<Runtime>;

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
	type Event = TestEvent;
	type Balance = Balance;
	type AuctionId = AuctionId;
	type Handler = Handler;
	type WeightInfo = ();
}
pub type AuctionModule = Module<Runtime>;

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
		let t = frame_system::GenesisConfig::default()
			.build_storage::<Runtime>()
			.unwrap();

		t.into()
	}
}
