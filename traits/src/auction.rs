use codec::FullCodec;
use rstd::fmt::Debug;
use sr_primitives::traits::MaybeSerializeDeserialize;

pub struct AuctionInfo<AccountId, Balance, BlockNumber> {
	pub bid: Option<(AccountId, Balance)>,
	pub end: Option<BlockNumber>,
}

pub trait Auction<AccountId, Balance, BlockNumber> {
	type AuctionId: FullCodec + Copy + MaybeSerializeDeserialize + Debug;

	fn auction_info(id: Self::AuctionId) -> Option<AuctionInfo<AccountId, Balance, BlockNumber>>;
	fn update_auction(id: Self::AuctionId, info: AuctionInfo<AccountId, Balance, BlockNumber>);
	fn new_auction(start: BlockNumber, end: Option<BlockNumber>) -> Self::AuctionId;
}

pub struct OnNewBidResult<BlockNumber> {
	pub accept_bid: bool,
	/// `None` means don't change, `Some(None)` means no more auction end time, `Some(Some(number))` means set auction end time to this block
	pub auction_end: Option<Option<BlockNumber>>,
}

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
	fn on_auction_ended(id: AuctionId, winner: Option<(AccountId, Balance)>);
}
