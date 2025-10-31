pub use crate::*;

use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;

/// Helper trait for benchmarking.
pub trait BenchmarkHelper<BlockNumber, AccountId, Balance> {
	fn setup_bid() -> Option<(AccountId, Balance)>;
	fn setup_on_finalize(rand: u32) -> Option<BlockNumber>;
}

impl<BlockNumber, AccountId, Balance> BenchmarkHelper<BlockNumber, AccountId, Balance> for () {
	fn setup_bid() -> Option<(AccountId, Balance)> {
		None
	}
	fn setup_on_finalize(_rand: u32) -> Option<BlockNumber> {
		None
	}
}

pub struct BaseBenchmarkHelper<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> BenchmarkHelper<BlockNumberFor<T>, T::AccountId, T::Balance> for BaseBenchmarkHelper<T> {
	fn setup_bid() -> Option<(T::AccountId, T::Balance)> {
		let end_block: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number() + 10u32.into();
		assert_ok!(Pallet::<T>::new_auction(
			frame_system::Pallet::<T>::block_number(),
			Some(end_block),
		));

		let auction_id: T::AuctionId = 0u32.into();
		let pre_bidder: T::AccountId = account("pre_bidder", 0, 0);
		let pre_bid_price: T::Balance = 10_000u32.into();

		assert_ok!(Pallet::<T>::bid(
			RawOrigin::Signed(pre_bidder).into(),
			auction_id,
			pre_bid_price
		));

		let bidder: T::AccountId = account("bidder", 0, 0);
		let bid_price: T::Balance = 20_000u32.into();

		Some((bidder, bid_price))
	}

	fn setup_on_finalize(rand: u32) -> Option<BlockNumberFor<T>> {
		let end_block: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number() + 10u32.into();

		for _auction_id in 0..rand {
			assert_ok!(Pallet::<T>::new_auction(
				frame_system::Pallet::<T>::block_number(),
				Some(end_block),
			));
		}
		Some(end_block)
	}
}

#[benchmarks]
mod benchmarks {
	use super::*;

	// `bid` a collateral auction, worst cases:
	// there's bidder before and bid price will exceed target amount
	#[benchmark]
	fn bid() {
		let auction_id: T::AuctionId = 0u32.into();
		let (bidder, bid_price) = T::BenchmarkHelper::setup_bid().unwrap();

		#[extrinsic_call]
		_(RawOrigin::Signed(bidder.clone()), auction_id, bid_price);

		frame_system::Pallet::<T>::assert_last_event(
			Event::Bid {
				auction_id,
				bidder: bidder,
				amount: bid_price,
			}
			.into(),
		);
	}

	#[benchmark]
	fn on_finalize(c: Liner<1, 100>) {
		let end_block = T::BenchmarkHelper::setup_on_finalize(c).unwrap();

		#[block]
		{
			Pallet::<T>::on_finalize(end_block);
		}
	}

	impl_benchmark_test_suite! {
		Pallet,
		crate::mock::ExtBuilder::default().build(),
		crate::mock::Runtime,
	}
}
