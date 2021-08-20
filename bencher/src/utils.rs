#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::vec::Vec;

#[cfg(feature = "std")]
use super::colorize::red_bold;
#[cfg(feature = "std")]
use super::redundant_meter::RedundantMeter;
#[cfg(feature = "std")]
use super::tracker::ChangesTrackerExt;
#[cfg(feature = "std")]
use sp_externalities::ExternalitiesExt;

#[cfg(feature = "std")]
thread_local! {
	static REDUNDANT_METER: std::cell::RefCell<RedundantMeter> = std::cell::RefCell::new(RedundantMeter::default());
}

#[sp_runtime_interface::runtime_interface]
pub trait Bench {
	fn panic(str: Vec<u8>) {
		let msg = String::from_utf8_lossy(&str);
		eprintln!("{}", red_bold(&msg));
		std::process::exit(-1);
	}

	fn entering_method() -> Vec<u8> {
		REDUNDANT_METER.with(|x| x.borrow_mut().entering_method())
	}

	fn leaving_method(identifier: Vec<u8>) {
		REDUNDANT_METER.with(|x| x.borrow_mut().leaving_method(identifier));
	}

	fn finalized_results(elapsed: u128) -> (u128, u32, u32) {
		let (reads, _, writes, _) = frame_benchmarking::benchmarking::read_write_count();
		let redundant = REDUNDANT_METER.with(|x| x.borrow_mut().take_results());

		let elapsed = elapsed.saturating_sub(redundant.elapsed);
		let reads = reads.saturating_sub(redundant.reads);
		let writes = writes.saturating_sub(redundant.writes);

		(elapsed, reads, writes)
	}

	fn reset() {
		REDUNDANT_METER.with(|x| {
			x.borrow_mut().reset();
		});
	}

	fn storage_changes_count(&mut self) -> u32 {
		let tracker = &***self
			.extension::<ChangesTrackerExt>()
			.expect("No `changes_tracker` associated for the current context!");
		tracker.changes_count()
	}
}
