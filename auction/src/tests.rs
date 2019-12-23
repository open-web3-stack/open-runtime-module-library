//! Unit tests for the tokens module.

#![cfg(test)]

use super::*;
use frame_support::assert_ok;
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
