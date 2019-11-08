#![cfg_attr(not(feature = "std"), no_std)]

use rstd::result;
use sr_primitives::traits::{MaybeSerializeDeserialize, Member, SimpleArithmetic};
use srml_support::{decl_error, decl_event, decl_module, decl_storage, ensure, Parameter};
use srml_system::{self as system, ensure_signed};

use traits::{AuctionInfo, Auction, AuctionHandler, OnNewBidResult};

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type Balance: Parameter + Member + SimpleArithmetic + Default + Copy + MaybeSerializeDeserialize;
	type AuctionId: Parameter + Member + SimpleArithmetic + Default + Copy + MaybeSerializeDeserialize;
	type Handler: AuctionHandler<Self::AccountId, Self::Balance, Self::BlockNumber, Self::AuctionId>;
}

decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId,
		<T as Trait>::Balance,
		<T as Trait>::AuctionId,
	{
		Dummy(AccountId),
	}
);

decl_storage! {
	trait Store for Module<T: Trait> as Auction {
		pub Auctions get(fn auctions): map T::AuctionId => AuctionInfo<T::AccountId, T::Balance, T::BlockNumber>;
		pub AuctionsCount get(fn auctions_count): u64;
	}
}



decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		pub fn bid(origin, id: AuctionId, value: Balance) -> Result {
			let from = ensure_signed(origin)?;
			
			ensure!(
				<Auctions<T>>::exists(id),
				Error::AuctionNotExist,
			);
			let now = <system::Module<T>>::block_number();
			let mut auction = Self::auctions(id);
			if let Some(end_blocknumber) = auction.end {
				Error::AuctionEnded
			}
			let result = T::Handler::on_new_bid(now, id, (AccountId.clone(), value), Some((auction.AccountId.clone(), Balance)));

			if !result.accept_bid {
				Error::BidNotAccepted
			}

			auction.bid = (from.clone(), value);
			if let Some(new_end) = result.auction_end {
				auction.end = new_end;
			}
			
			<Auctions<T>>::insert(id, auction);		
			Ok(())
		}
	}
}

decl_error! {
	/// Error for auction module.
	pub enum Error {
		AuctionNotExist,
		AuctionEnded,
		BidNotAccepted,
	}
}

impl<T: Trait> Module<T> {}

impl<T: Trait> Auction<T::AccountId, T::BlockNumber> for Module<T> {
	type AuctionId = T::AuctionId;
	type Balance = T::Balance;
	
	fn auction_info(id: Self::AuctionId) -> AuctionInfo<T::AccountId, Self::Balance, T::BlockNumber> {
		Self::auctions(id)
	}

	fn update_auction(id: Self::AuctionId, info: AuctionInfo<AccountId, Balance, BlockNumber>) -> result::Result<(), Self::Error> {
		ensure!(
			<Auctions<T>>::exists(id),
			Error::AuctionNotExist,
		);

		<Auctions<T>>::insert(id, info);
		Ok(())
	}

	fn new_auction(start: T::BlockNumber, end: Option<T::BlockNumber>) -> Self::AuctionId {
		let auction = AuctionInfo{
			bid: None,
			end: end
		};
		let auction_id = T::AuctionId::from(<AuctionsCount<T>>::get());

		<AuctionsCount<T>>::mutate(|n| *n += 1);
		<Auctions<T>>::insert(auction_id, auction);
		auction_id
	}
}