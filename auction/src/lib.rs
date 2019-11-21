#![cfg_attr(not(feature = "std"), no_std)]

use orml_utilities::{LinkedItem, LinkedList};
use palette_support::{decl_error, decl_event, decl_module, decl_storage, dispatch::Result, ensure, Parameter};
use palette_system::{self as system, ensure_signed};
use rstd::result;
use sr_primitives::traits::{MaybeSerializeDeserialize, Member, SimpleArithmetic};

use orml_traits::{Auction, AuctionHandler, AuctionInfo};

mod mock;
mod tests;

pub trait Trait: palette_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as palette_system::Trait>::Event>;
	type Balance: Parameter + Member + SimpleArithmetic + Default + Copy + MaybeSerializeDeserialize;
	type AuctionId: Parameter + Member + SimpleArithmetic + Default + Copy + MaybeSerializeDeserialize;
	type Handler: AuctionHandler<Self::AccountId, Self::Balance, Self::BlockNumber, Self::AuctionId>;
}

type AuctionEndTimeList<T> =
	LinkedList<AuctionEndTime<T>, <T as palette_system::Trait>::BlockNumber, <T as Trait>::AuctionId>;

decl_event!(
	pub enum Event<T> where
		<T as palette_system::Trait>::AccountId,
		<T as Trait>::Balance,
		<T as Trait>::AuctionId,
	{
		Bid(AuctionId, AccountId, Balance),
	}
);

decl_storage! {
	trait Store for Module<T: Trait> as Auction {
		pub Auctions get(fn auctions): map T::AuctionId => Option<AuctionInfo<T::AccountId, T::Balance, T::BlockNumber>>;
		pub AuctionsCount get(fn auctions_count): T::AuctionId;
		pub AuctionEndTime get(fn auction_end_time): map(T::BlockNumber, Option<T::AuctionId>) => Option<LinkedItem<T::AuctionId>>;
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		pub fn bid(origin, id: T::AuctionId, value: T::Balance) -> Result {
			let from = ensure_signed(origin)?;

			let mut auction = <Auctions<T>>::get(id).ok_or(Error::AuctionNotExist)?;

			let block_number = <palette_system::Module<T>>::block_number();

			// make sure auction is started
			ensure!(block_number >= auction.start, Error::AuctionNotStarted.into());

			if let Some(ref current_bid) = auction.bid {
				ensure!(value > current_bid.1, Error::InvalidBidPrice.into());
			} else {
				ensure!(value > 0.into(), Error::InvalidBidPrice.into());
			}
			let bid_result = T::Handler::on_new_bid(
				block_number,
				id,
				(from.clone(), value),
				auction.bid.clone(),
			);

			ensure!(bid_result.accept_bid, Error::BidNotAccepted.into());
			if let Some(new_end) = bid_result.auction_end {
				if let Some(old_end_block) = auction.end {
					<AuctionEndTimeList<T>>::remove(&old_end_block, id);
				}
				if let Some(new_end_block) = new_end {
					<AuctionEndTimeList<T>>::append(&new_end_block, id);
				}
				auction.end = new_end;
			}
			auction.bid = Some((from.clone(), value));
			<Auctions<T>>::insert(id, auction);
			Self::deposit_event(RawEvent::Bid(id, from, value));
			Ok(())
		}

		fn on_finalize(now: T::BlockNumber) {
			let head_key: Option<T::AuctionId> = None;
			if let Some(mut head_item) = <AuctionEndTime<T>>::get((now, head_key)) {
				while let Some(auction_id) = head_item.next {
					if let Some(auction) = Self::auctions(auction_id) {
						T::Handler::on_auction_ended(auction_id, auction.bid);
						<Auctions<T>>::remove(auction_id);
					}
					head_item = <AuctionEndTime<T>>::get((now, Some(auction_id))).unwrap_or_else(|| LinkedItem {
							prev: None,
							next: None,
						});
					<AuctionEndTime<T>>::remove((now, Some(auction_id)));
				}

				<AuctionEndTime<T>>::remove((now, head_key));
			}
		}
	}
}

decl_error! {
	/// Error for auction module.
	pub enum Error {
		AuctionNotExist,
		AuctionNotStarted,
		BidNotAccepted,
		InvalidBidPrice,
	}
}

impl<T: Trait> Module<T> {}

impl<T: Trait> Auction<T::AccountId, T::BlockNumber> for Module<T> {
	type AuctionId = T::AuctionId;
	type Balance = T::Balance;
	type Error = Error;

	fn auction_info(id: Self::AuctionId) -> Option<AuctionInfo<T::AccountId, Self::Balance, T::BlockNumber>> {
		Self::auctions(id)
	}

	fn update_auction(
		id: Self::AuctionId,
		info: AuctionInfo<T::AccountId, Self::Balance, T::BlockNumber>,
	) -> result::Result<(), Self::Error> {
		let auction = <Auctions<T>>::get(id).ok_or(Error::AuctionNotExist)?;
		if let Some(old_end) = auction.end {
			<AuctionEndTimeList<T>>::remove(&old_end, id);
		}
		if let Some(new_end) = info.end {
			<AuctionEndTimeList<T>>::append(&new_end, id);
		}
		<Auctions<T>>::insert(id, info);
		Ok(())
	}

	fn new_auction(start: T::BlockNumber, end: Option<T::BlockNumber>) -> Self::AuctionId {
		let auction = AuctionInfo { bid: None, start, end };
		let auction_id = Self::auctions_count();
		<AuctionsCount<T>>::mutate(|n| *n += Self::AuctionId::from(1));
		<Auctions<T>>::insert(auction_id, auction);
		if let Some(end_block) = end {
			<AuctionEndTimeList<T>>::append(&end_block, auction_id);
		}

		auction_id
	}
}
