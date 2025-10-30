pub use crate::*;

use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::{EventRecord, RawOrigin};

/// Helper trait for benchmarking.
pub trait BenchmarkHelper<BlockNumber> {
	fn setup_bid();
	fn setup_on_finalize(rand: u32) -> Option<BlockNumber>;
}

impl<BlockNumber> BenchmarkHelper<BlockNumber> for () {
	fn setup_bid() {}
	fn setup_on_finalize(_rand: u32) -> Option<BlockNumber> {
		None
	}
}

pub struct BaseBenchmarkHelper<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> BenchmarkHelper<BlockNumberFor<T>> for BaseBenchmarkHelper<T> {
	fn setup_bid() {
		let end_block: BlockNumberFor<T> = frame_system::Pallet::<T>::block_number() + 10u32.into();
		assert_ok!(Pallet::<T>::new_auction(
			frame_system::Pallet::<T>::block_number(),
			Some(end_block),
		));

		let auction_id: T::AuctionId = 0u32.into();
		let bidder: T::AccountId = account("pre_bidder", 0, 0);
		let bid_price: T::Balance = 10_000u32.into();

		assert_ok!(Pallet::<T>::bid(
			RawOrigin::Signed(bidder.clone()).into(),
			auction_id,
			bid_price
		));
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

fn assert_last_event<T: Config>(generic_event: <T as frame_system::Config>::RuntimeEvent) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::RuntimeEvent = generic_event.into();
	// compare to the last event record
	let EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

#[benchmarks]
mod benchmarks {
	use super::*;

	// `bid` a collateral auction, worst cases:
	// there's bidder before and bid price will exceed target amount
	#[benchmark]
	fn bid() {
		T::BenchmarkHelper::setup_bid();

		let auction_id: T::AuctionId = 0u32.into();
		let bidder: T::AccountId = account("bidder", 0, 0);
		let bid_price: T::Balance = 20_000u32.into();

		#[extrinsic_call]
		_(RawOrigin::Signed(bidder.clone()), auction_id, bid_price);

		assert_last_event::<T>(
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
