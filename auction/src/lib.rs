//! # Auction
//!
//! ## Overview
//!
//! This module provides a basic abstraction to implement on-chain auctioning
//! feature.
//!
//! The auction logic can be customized by implement and supplying
//! `AuctionHandler` trait.

#![cfg_attr(not(feature = "std"), no_std)]
// Disable the following two lints since they originate from an external macro (namely decl_storage)
#![allow(clippy::string_lit_as_bytes)]

use frame_support::{
	decl_error, decl_event, decl_module, decl_storage, ensure, weights::Weight, IterableStorageDoubleMap, Parameter,
};
use frame_system::ensure_signed;
use orml_traits::{Auction, AuctionHandler, AuctionInfo, Change};
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, Bounded, MaybeSerializeDeserialize, Member, One, Zero},
	DispatchError, DispatchResult,
};
use sp_std::result;

mod default_weight;
mod mock;
mod tests;

pub trait WeightInfo {
	fn bid_collateral_auction() -> Weight;
	fn bid_surplus_auction() -> Weight;
	fn bid_debit_auction() -> Weight;
	fn on_finalize(c: u32) -> Weight;
}

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

	/// The balance type for bidding
	type Balance: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + MaybeSerializeDeserialize;

	/// The auction ID type
	type AuctionId: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + MaybeSerializeDeserialize + Bounded;

	/// The `AuctionHandler` that allow custom bidding logic and handles auction
	/// result
	type Handler: AuctionHandler<Self::AccountId, Self::Balance, Self::BlockNumber, Self::AuctionId>;

	/// Weight information for extrinsics in this module.
	type WeightInfo: WeightInfo;
}

decl_event!(
	pub enum Event<T> where
		<T as frame_system::Trait>::AccountId,
		<T as Trait>::Balance,
		<T as Trait>::AuctionId,
	{
		/// A bid is placed. [auction_id, bidder, bidding_amount]
		Bid(AuctionId, AccountId, Balance),
	}
);

decl_storage! {
	trait Store for Module<T: Trait> as Auction {
		/// Stores on-going and future auctions. Closed auction are removed.
		pub Auctions get(fn auctions): map hasher(twox_64_concat) T::AuctionId => Option<AuctionInfo<T::AccountId, T::Balance, T::BlockNumber>>;

		/// Track the next auction ID.
		pub AuctionsIndex get(fn auctions_index): T::AuctionId;

		/// Index auctions by end time.
		pub AuctionEndTime get(fn auction_end_time): double_map hasher(twox_64_concat) T::BlockNumber, hasher(twox_64_concat) T::AuctionId => Option<()>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		/// Bid an auction.
		///
		/// The dispatch origin for this call must be `Signed` by the transactor.
		///
		/// # <weight>
		/// - Preconditions:
		///     - T::Handler is module_auction_manager of Acala
		///     - Indirectly needs orml_currencies and module_cdp_treasury of Acala
		/// - Complexity: `O(1)`
		/// - Db reads:
		///     - collateral auction:
		///             - best cases: 7
		///             - worst cases: 14
		///     - surplus auction:
		///             - best cases: 5
		///             - worst cases: 6
		///     - debit auction:
		///             - best cases: 8
		///             - worst cases: 7
		/// - Db writes:
		///     - collateral auction:
		///             - best cases: 7
		///             - worst cases: 14
		///     - surplus auction:
		///             - best cases: 3
		///             - worst cases: 5
		///     - debit auction:
		///             - best cases: 8
		///             - worst cases: 8
		/// -------------------
		/// Base Weight:
		///     - collateral auction:
		///             - best cases: 134 µs
		///             - worst cases: 300.4 µs
		///     - surplus auction:
		///             - best cases: 97.9 µs
		///             - worst cases: 157.6 µs
		///     - debit auction:
		///             - best cases: 140.7 µs
		///             - worst cases: 142.8 µs
		/// # </weight>
		#[weight = T::WeightInfo::bid_collateral_auction()]
		pub fn bid(origin, id: T::AuctionId, #[compact] value: T::Balance) {
			let from = ensure_signed(origin)?;

			<Auctions<T>>::try_mutate_exists(id, |auction| -> DispatchResult {
				let mut auction = auction.as_mut().ok_or(Error::<T>::AuctionNotExist)?;

				let block_number = <frame_system::Module<T>>::block_number();

				// make sure auction is started
				ensure!(block_number >= auction.start, Error::<T>::AuctionNotStarted);

				if let Some(ref current_bid) = auction.bid {
					ensure!(value > current_bid.1, Error::<T>::InvalidBidPrice);
				} else {
					ensure!(!value.is_zero(), Error::<T>::InvalidBidPrice);
				}
				let bid_result = T::Handler::on_new_bid(
					block_number,
					id,
					(from.clone(), value),
					auction.bid.clone(),
				);

				ensure!(bid_result.accept_bid, Error::<T>::BidNotAccepted);
				match bid_result.auction_end_change {
					Change::NewValue(new_end) => {
						if let Some(old_end_block) = auction.end {
							<AuctionEndTime<T>>::remove(&old_end_block, id);
						}
						if let Some(new_end_block) = new_end {
							<AuctionEndTime<T>>::insert(&new_end_block, id, ());
						}
						auction.end = new_end;
					},
					Change::NoChange => {},
				}
				auction.bid = Some((from.clone(), value));

				Ok(())
			})?;

			Self::deposit_event(RawEvent::Bid(id, from, value));
		}

		/// dummy `on_initialize` to return the weight used in `on_finalize`.
		fn on_initialize(now: T::BlockNumber) -> Weight {
			T::WeightInfo::on_finalize(<AuctionEndTime<T>>::iter_prefix(&now).count() as u32)
		}

		fn on_finalize(now: T::BlockNumber) {
			Self::_on_finalize(now);
		}
	}
}

decl_error! {
	/// Error for auction module.
	pub enum Error for Module<T: Trait> {
		AuctionNotExist,
		AuctionNotStarted,
		BidNotAccepted,
		InvalidBidPrice,
		NoAvailableAuctionId,
	}
}

impl<T: Trait> Module<T> {
	fn _on_finalize(now: T::BlockNumber) {
		for (auction_id, _) in <AuctionEndTime<T>>::drain_prefix(&now) {
			if let Some(auction) = <Auctions<T>>::take(&auction_id) {
				T::Handler::on_auction_ended(auction_id, auction.bid);
			}
		}
	}
}

impl<T: Trait> Auction<T::AccountId, T::BlockNumber> for Module<T> {
	type AuctionId = T::AuctionId;
	type Balance = T::Balance;

	fn auction_info(id: Self::AuctionId) -> Option<AuctionInfo<T::AccountId, Self::Balance, T::BlockNumber>> {
		Self::auctions(id)
	}

	fn update_auction(
		id: Self::AuctionId,
		info: AuctionInfo<T::AccountId, Self::Balance, T::BlockNumber>,
	) -> DispatchResult {
		let auction = <Auctions<T>>::get(id).ok_or(Error::<T>::AuctionNotExist)?;
		if let Some(old_end) = auction.end {
			<AuctionEndTime<T>>::remove(&old_end, id);
		}
		if let Some(new_end) = info.end {
			<AuctionEndTime<T>>::insert(&new_end, id, ());
		}
		<Auctions<T>>::insert(id, info);
		Ok(())
	}

	fn new_auction(
		start: T::BlockNumber,
		end: Option<T::BlockNumber>,
	) -> result::Result<Self::AuctionId, DispatchError> {
		let auction = AuctionInfo { bid: None, start, end };
		let auction_id = <AuctionsIndex<T>>::try_mutate(|n| -> result::Result<Self::AuctionId, DispatchError> {
			let id = *n;
			ensure!(id != Self::AuctionId::max_value(), Error::<T>::NoAvailableAuctionId);
			*n += One::one();
			Ok(id)
		})?;
		<Auctions<T>>::insert(auction_id, auction);
		if let Some(end_block) = end {
			<AuctionEndTime<T>>::insert(&end_block, auction_id, ());
		}

		Ok(auction_id)
	}

	fn remove_auction(id: Self::AuctionId) {
		if let Some(auction) = <Auctions<T>>::take(&id) {
			if let Some(end_block) = auction.end {
				<AuctionEndTime<T>>::remove(end_block, id);
			}
		}
	}
}
