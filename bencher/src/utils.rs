use sp_std::vec::Vec;

#[cfg(feature = "std")]
use super::colorize::red_bold;
#[cfg(feature = "std")]
use super::tracker::BenchTrackerExt;
#[cfg(feature = "std")]
use sp_externalities::ExternalitiesExt;

#[sp_runtime_interface::runtime_interface]
pub trait Bench {
	fn print_error(str: Vec<u8>) {
		let msg = String::from_utf8_lossy(&str);
		eprintln!("{}", red_bold(&msg));
	}

	fn instant(&mut self) {
		let tracker = &***self
			.extension::<BenchTrackerExt>()
			.expect("No `changes_tracker` associated for the current context!");
		tracker.instant();
	}

	fn elapsed(&mut self) -> u128 {
		let tracker = &***self
			.extension::<BenchTrackerExt>()
			.expect("No `changes_tracker` associated for the current context!");
		tracker.elapsed()
	}

	fn before_block(&mut self) {
		let tracker = &***self
			.extension::<BenchTrackerExt>()
			.expect("No `changes_tracker` associated for the current context!");
		tracker.before_block();
	}

	fn after_block(&mut self) {
		let tracker = &***self
			.extension::<BenchTrackerExt>()
			.expect("No `changes_tracker` associated for the current context!");
		tracker.after_block();
	}

	fn redundant_time(&mut self) -> u128 {
		let tracker = &***self
			.extension::<BenchTrackerExt>()
			.expect("No `changes_tracker` associated for the current context!");
		tracker.redundant_time()
	}

	fn reset_redundant(&mut self) {
		let tracker = &***self
			.extension::<BenchTrackerExt>()
			.expect("No `changes_tracker` associated for the current context!");
		tracker.reset_redundant();
	}

	fn read_written_keys(&mut self) -> Vec<u8> {
		let tracker = &***self
			.extension::<BenchTrackerExt>()
			.expect("No `changes_tracker` associated for the current context!");
		tracker.read_written_keys()
	}
}
