use codec::FullCodec;
use codec::{Decode, Encode};
use rstd::{fmt::Debug, result};
use sr_primitives::{
	traits::{MaybeSerializeDeserialize, SimpleArithmetic},
	RuntimeDebug,
};

/// Auction info.
#[cfg_attr(feature = "std", derive(PartialEq, Eq))]
#[derive(Encode, Decode, RuntimeDebug)]
pub struct AuctionInfo<AccountId, Balance, BlockNumber> {
	/// Current bidder and bid price.
	pub bid: Option<(AccountId, Balance)>,
	/// Define which block this auction will be started.
	pub start: BlockNumber,
	/// Define which block this auction will be ended.
	pub end: Option<BlockNumber>,
}

/// Abstraction over a simple auction system.
pub trait Auction<AccountId, BlockNumber> {
	/// The id of an AuctionInfo
	type AuctionId: FullCodec + Default + Copy + MaybeSerializeDeserialize + Debug;
	/// The price to bid.
	type Balance: SimpleArithmetic + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;
	/// The error type.
	type Error: Into<&'static str>;

	/// The auction info of `id`
	fn auction_info(id: Self::AuctionId) -> Option<AuctionInfo<AccountId, Self::Balance, BlockNumber>>;
	/// Update the auction info of `id` with `info`
	fn update_auction(
		id: Self::AuctionId,
		info: AuctionInfo<AccountId, Self::Balance, BlockNumber>,
	) -> result::Result<(), Self::Error>;
	/// Create new auction with specific startblock and endblock, return the id of the auction
	fn new_auction(start: BlockNumber, end: Option<BlockNumber>) -> Self::AuctionId;
}

/// The result of bid handling.
pub struct OnNewBidResult<BlockNumber> {
	/// Indicates if the bid was accepted
	pub accept_bid: bool,
	/// `None` means don't change, `Some(None)` means no more auction end time, `Some(Some(number))` means set auction end time to this block
	pub auction_end: Option<Option<BlockNumber>>,
}

/// Hooks for auction to handle bids.
pub trait AuctionHandler<AccountId, Balance, BlockNumber, AuctionId> {
	/// Called when new bid is received.
	/// The return value deteermine if the bid should be accepted and update auction end time.
	/// Implementation should reserve money from current winner and refund previous winner.
	fn on_new_bid(
		now: BlockNumber,
		id: AuctionId,
		new_bid: (AccountId, Balance),
		last_bid: Option<(AccountId, Balance)>,
	) -> OnNewBidResult<BlockNumber>;
	/// End an auction with `winner`
	fn on_auction_ended(id: AuctionId, winner: Option<(AccountId, Balance)>);
}
