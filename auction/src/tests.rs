//! Unit tests for the tokens module.

#![cfg(test)]

use super::*;
use frame_support::assert_ok;
use mock::{AuctionModule, ExtBuilder, Runtime, ALICE};
use sp_runtime::traits::OnFinalize;

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
		assert_eq!(AuctionModule::new_auction(1, Some(100)), 0);
		assert_ok!(AuctionModule::bid(Some(ALICE).into(), 0, 20));
		assert_eq!(
			AuctionModule::auction_info(0),
			Some(AuctionInfo {
				bid: Some((ALICE, 20)),
				start: 1,
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
		assert_eq!(AuctionModule::auction_end_time((100, Some(0))).is_some(), true);
		AuctionModule::remove_auction(0);
		assert_eq!(AuctionModule::auctions(0), None);
		assert_eq!(AuctionModule::auction_end_time((100, Some(0))), None);
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
		AuctionModule::on_finalize(50);
		assert_eq!(AuctionModule::auctions(0).is_some(), true);
		assert_eq!(AuctionModule::auctions(1).is_some(), false);
		AuctionModule::on_finalize(100);
		assert_eq!(AuctionModule::auctions(0).is_some(), false);
		assert_eq!(AuctionModule::auctions(1).is_some(), false);
	});
}
