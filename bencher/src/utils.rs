use sp_std::vec::Vec;

#[cfg(feature = "std")]
use super::colorize::{cyan, red_bold, yellow_bold};
#[cfg(feature = "std")]
use super::tracker::BenchTrackerExt;
#[cfg(feature = "std")]
use codec::Decode;
#[cfg(feature = "std")]
use sp_externalities::ExternalitiesExt;

#[sp_runtime_interface::runtime_interface]
pub trait Bench {
	fn print_error(message: Vec<u8>) {
		let msg = String::from_utf8_lossy(&message);
		eprintln!("{}", red_bold(&msg));
	}

	fn print_warnings(&mut self, method: Vec<u8>) {
		let tracker = &***self
			.extension::<BenchTrackerExt>()
			.expect("No `bench_tracker` associated for the current context!");
		if tracker.has_warn_child_prefix_removal() {
			let method_name = <String as Decode>::decode(&mut &method[..]).unwrap();
			println!(
				"{} {} kills prefix and/or child storage which are ignored.",
				yellow_bold("WARNING:"),
				cyan(&method_name)
			);
		}
	}

	fn print_info(&mut self, message: Vec<u8>) {
		let msg = String::from_utf8_lossy(&message);
		println!("{}", msg);
	}

	fn instant(&mut self) {
		let tracker = &***self
			.extension::<BenchTrackerExt>()
			.expect("No `bench_tracker` associated for the current context!");
		tracker.instant();
	}

	fn elapsed(&mut self) -> u128 {
		let tracker = &***self
			.extension::<BenchTrackerExt>()
			.expect("No `bench_tracker` associated for the current context!");
		tracker.elapsed()
	}

	fn before_block(&mut self) {
		let tracker = &***self
			.extension::<BenchTrackerExt>()
			.expect("No `bench_tracker` associated for the current context!");
		tracker.before_block();
	}

	fn after_block(&mut self) {
		let tracker = &***self
			.extension::<BenchTrackerExt>()
			.expect("No `bench_tracker` associated for the current context!");
		tracker.after_block();
	}

	fn redundant_time(&mut self) -> u128 {
		let tracker = &***self
			.extension::<BenchTrackerExt>()
			.expect("No `bench_tracker` associated for the current context!");
		tracker.redundant_time()
	}

	fn prepare(&mut self) {
		let tracker = &***self
			.extension::<BenchTrackerExt>()
			.expect("No `bench_tracker` associated for the current context!");
		tracker.prepare();
	}

	fn read_written_keys(&mut self) -> Vec<u8> {
		let tracker = &***self
			.extension::<BenchTrackerExt>()
			.expect("No `bench_tracker` associated for the current context!");
		tracker.read_written_keys()
	}

	fn whitelist(&mut self, key: Vec<u8>, read: bool, write: bool) {
		let tracker = &***self
			.extension::<BenchTrackerExt>()
			.expect("No `bench_tracker` associated for the current context!");
		tracker.whitelist(key, read, write);
	}

	fn reset(&mut self) {
		let tracker = &***self
			.extension::<BenchTrackerExt>()
			.expect("No `bench_tracker` associated for the current context!");
		tracker.reset();
	}
}
