//! Unit tests for the tokens module.

#![cfg(test)]

use super::*;
use frame_support::{assert_ok, traits::OnFinalize};
use mock::{AuctionModule, ExtBuilder, Runtime, ALICE};

#[test]
fn new_auction_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(AuctionModule::new_auction(10, Some(100)), 0);
	});
}

#[test]
fn update_auction_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(AuctionModule::new_auction(10, Some(100)), 0);
		assert_ok!(AuctionModule::update_auction(
			0,
			AuctionInfo {
				bid: Some((ALICE, 100)),
				start: 10,
				end: Some(100)
			}
		));
	});
}

#[test]
fn auction_info_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(AuctionModule::new_auction(10, Some(100)), 0);
		assert_eq!(
			AuctionModule::auction_info(0),
			Some(AuctionInfo {
				bid: None,
				start: 10,
				end: Some(100)
			})
		);
	});
}

#[test]
fn bid_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(AuctionModule::new_auction(0, Some(100)), 0);
		assert_ok!(AuctionModule::bid(Some(ALICE).into(), 0, 20));
		assert_eq!(
			AuctionModule::auction_info(0),
			Some(AuctionInfo {
				bid: Some((ALICE, 20)),
				start: 0,
				end: Some(100)
			})
		);
	});
}

#[test]
fn bid_should_fail() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(AuctionModule::new_auction(10, Some(100)), 0);
		assert_eq!(
			AuctionModule::bid(Some(ALICE).into(), 0, 20),
			Err(Error::<Runtime>::AuctionNotStarted.into())
		);
	});
}

#[test]
fn remove_auction_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(AuctionModule::new_auction(10, Some(100)), 0);
		assert_eq!(AuctionModule::auctions_index(), 1);
		assert_eq!(AuctionModule::auctions(0).is_some(), true);
		assert_eq!(AuctionModule::auction_end_time(100, 0), Some(true));
		AuctionModule::remove_auction(0);
		assert_eq!(AuctionModule::auctions(0), None);
		assert_eq!(AuctionModule::auction_end_time(100, 0), None);
	});
}

#[test]
fn cleanup_auction_should_work() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(AuctionModule::new_auction(10, Some(100)), 0);
		assert_eq!(AuctionModule::auctions_index(), 1);
		assert_eq!(AuctionModule::new_auction(10, Some(50)), 1);
		assert_eq!(AuctionModule::auctions_index(), 2);
		assert_eq!(AuctionModule::auctions(0).is_some(), true);
		assert_eq!(AuctionModule::auctions(1).is_some(), true);

		assert_eq!(<AuctionEndTime<Runtime>>::iter_prefix(0).count(), 0);
		assert_eq!(<AuctionEndTime<Runtime>>::iter_prefix(50).count(), 1);
		assert_eq!(<AuctionEndTime<Runtime>>::iter_prefix(100).count(), 1);

		AuctionModule::on_finalize(50);
		assert_eq!(AuctionModule::auctions(0).is_some(), true);
		assert_eq!(AuctionModule::auctions(1).is_some(), false);
		assert_eq!(<AuctionEndTime<Runtime>>::iter_prefix(0).count(), 0);
		assert_eq!(<AuctionEndTime<Runtime>>::iter_prefix(50).count(), 0);
		assert_eq!(<AuctionEndTime<Runtime>>::iter_prefix(100).count(), 1);

		AuctionModule::on_finalize(100);
		assert_eq!(AuctionModule::auctions(0).is_some(), false);
		assert_eq!(AuctionModule::auctions(1).is_some(), false);
		assert_eq!(<AuctionEndTime<Runtime>>::iter_prefix(0).count(), 0);
		assert_eq!(<AuctionEndTime<Runtime>>::iter_prefix(50).count(), 0);
		assert_eq!(<AuctionEndTime<Runtime>>::iter_prefix(100).count(), 0);
	});
}
