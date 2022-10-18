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
		let method_name = <String as Decode>::decode(&mut &method[..]).unwrap();
		tracker.warnings().iter().for_each(|warning| {
			println!(
				"{} {} {}",
				yellow_bold("WARNING:"),
				cyan(&method_name),
				yellow_bold(&warning.to_string())
			);
		});
	}

	fn print_info(&mut self, message: Vec<u8>) {
		let msg = String::from_utf8_lossy(&message);
		println!("{}", msg);
	}

	fn start_timer(&mut self) {
		let tracker = &***self
			.extension::<BenchTrackerExt>()
			.expect("No `bench_tracker` associated for the current context!");
		tracker.prepare_next_run();
		tracker.instant();
	}

	fn end_timer(&mut self) -> u128 {
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

	fn init_bench(&mut self) {
		let tracker = &***self
			.extension::<BenchTrackerExt>()
			.expect("No `bench_tracker` associated for the current context!");
		tracker.reset();
	}

	fn count_clear_prefix(&mut self) {
		let tracker = &***self
			.extension::<BenchTrackerExt>()
			.expect("No `bench_tracker` associated for the current context!");
		tracker.count_clear_prefix();
	}
}
