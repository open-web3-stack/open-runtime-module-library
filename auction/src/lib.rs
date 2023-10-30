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
#![allow(clippy::unused_unit)]

use frame_support::pallet_prelude::*;
use frame_system::{ensure_signed, pallet_prelude::*};
use orml_traits::{Auction, AuctionHandler, AuctionInfo, Change};
use parity_scale_codec::MaxEncodedLen;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, Bounded, CheckedAdd, MaybeSerializeDeserialize, Member, One, Zero},
	DispatchError, DispatchResult,
};

mod mock;
mod tests;
mod weights;

pub use module::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The balance type for bidding.
		type Balance: Parameter
			+ Member
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;

		/// The auction ID type.
		type AuctionId: Parameter
			+ Member
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ Bounded
			+ parity_scale_codec::FullCodec
			+ parity_scale_codec::MaxEncodedLen;

		/// The `AuctionHandler` that allow custom bidding logic and handles
		/// auction result.
		type Handler: AuctionHandler<Self::AccountId, Self::Balance, BlockNumberFor<Self>, Self::AuctionId>;

		/// Weight information for extrinsics in this module.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		AuctionNotExist,
		AuctionNotStarted,
		BidNotAccepted,
		InvalidBidPrice,
		NoAvailableAuctionId,
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// A bid is placed
		Bid {
			auction_id: T::AuctionId,
			bidder: T::AccountId,
			amount: T::Balance,
		},
	}

	/// Stores on-going and future auctions. Closed auction are removed.
	#[pallet::storage]
	#[pallet::getter(fn auctions)]
	pub type Auctions<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AuctionId,
		AuctionInfo<T::AccountId, T::Balance, BlockNumberFor<T>>,
		OptionQuery,
	>;

	/// Track the next auction ID.
	#[pallet::storage]
	#[pallet::getter(fn auctions_index)]
	pub type AuctionsIndex<T: Config> = StorageValue<_, T::AuctionId, ValueQuery>;

	/// Index auctions by end time.
	#[pallet::storage]
	#[pallet::getter(fn auction_end_time)]
	pub type AuctionEndTime<T: Config> =
		StorageDoubleMap<_, Twox64Concat, BlockNumberFor<T>, Blake2_128Concat, T::AuctionId, (), OptionQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			T::WeightInfo::on_finalize(AuctionEndTime::<T>::iter_prefix(now).count() as u32)
		}

		fn on_finalize(now: BlockNumberFor<T>) {
			for (auction_id, _) in AuctionEndTime::<T>::drain_prefix(now) {
				if let Some(auction) = Auctions::<T>::take(auction_id) {
					T::Handler::on_auction_ended(auction_id, auction.bid);
				}
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Bid an auction.
		///
		/// The dispatch origin for this call must be `Signed` by the
		/// transactor.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::bid_collateral_auction())]
		pub fn bid(origin: OriginFor<T>, id: T::AuctionId, #[pallet::compact] value: T::Balance) -> DispatchResult {
			let from = ensure_signed(origin)?;

			Auctions::<T>::try_mutate_exists(id, |auction| -> DispatchResult {
				let auction = auction.as_mut().ok_or(Error::<T>::AuctionNotExist)?;

				let block_number = <frame_system::Pallet<T>>::block_number();

				// make sure auction is started
				ensure!(block_number >= auction.start, Error::<T>::AuctionNotStarted);

				if let Some(ref current_bid) = auction.bid {
					ensure!(value > current_bid.1, Error::<T>::InvalidBidPrice);
				} else {
					ensure!(!value.is_zero(), Error::<T>::InvalidBidPrice);
				}
				let bid_result = T::Handler::on_new_bid(block_number, id, (from.clone(), value), auction.bid.clone());

				ensure!(bid_result.accept_bid, Error::<T>::BidNotAccepted);
				match bid_result.auction_end_change {
					Change::NewValue(new_end) => {
						if let Some(old_end_block) = auction.end {
							AuctionEndTime::<T>::remove(old_end_block, id);
						}
						if let Some(new_end_block) = new_end {
							AuctionEndTime::<T>::insert(new_end_block, id, ());
						}
						auction.end = new_end;
					}
					Change::NoChange => {}
				}
				auction.bid = Some((from.clone(), value));

				Ok(())
			})?;

			Self::deposit_event(Event::Bid {
				auction_id: id,
				bidder: from,
				amount: value,
			});
			Ok(())
		}
	}
}

impl<T: Config> Auction<T::AccountId, BlockNumberFor<T>> for Pallet<T> {
	type AuctionId = T::AuctionId;
	type Balance = T::Balance;

	fn auction_info(id: Self::AuctionId) -> Option<AuctionInfo<T::AccountId, Self::Balance, BlockNumberFor<T>>> {
		Self::auctions(id)
	}

	fn update_auction(
		id: Self::AuctionId,
		info: AuctionInfo<T::AccountId, Self::Balance, BlockNumberFor<T>>,
	) -> DispatchResult {
		let auction = Auctions::<T>::get(id).ok_or(Error::<T>::AuctionNotExist)?;
		if let Some(old_end) = auction.end {
			AuctionEndTime::<T>::remove(old_end, id);
		}
		if let Some(new_end) = info.end {
			AuctionEndTime::<T>::insert(new_end, id, ());
		}
		Auctions::<T>::insert(id, info);
		Ok(())
	}

	fn new_auction(
		start: BlockNumberFor<T>,
		end: Option<BlockNumberFor<T>>,
	) -> sp_std::result::Result<Self::AuctionId, DispatchError> {
		let auction = AuctionInfo { bid: None, start, end };
		let auction_id =
			<AuctionsIndex<T>>::try_mutate(|n| -> sp_std::result::Result<Self::AuctionId, DispatchError> {
				let id = *n;
				*n = n.checked_add(&One::one()).ok_or(Error::<T>::NoAvailableAuctionId)?;
				Ok(id)
			})?;
		Auctions::<T>::insert(auction_id, auction);
		if let Some(end_block) = end {
			AuctionEndTime::<T>::insert(end_block, auction_id, ());
		}

		Ok(auction_id)
	}

	fn remove_auction(id: Self::AuctionId) {
		if let Some(auction) = Auctions::<T>::take(id) {
			if let Some(end_block) = auction.end {
				AuctionEndTime::<T>::remove(end_block, id);
			}
		}
	}
}
