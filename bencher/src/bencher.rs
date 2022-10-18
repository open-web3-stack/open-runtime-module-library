use codec::{Decode, Encode};
use sp_std::prelude::Vec;

#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, Debug)]
pub struct BenchResult {
	pub method: Vec<u8>,
	pub elapses: Vec<u128>,
	pub keys: Vec<u8>,
}

impl BenchResult {
	pub fn with_name(name: &str) -> Self {
		Self {
			method: name.as_bytes().to_vec(),
			..Default::default()
		}
	}
}

#[derive(Default)]
pub struct Bencher {
	pub current: BenchResult,
	pub results: Vec<BenchResult>,
}

#[inline]
fn black_box<T>(dummy: T) -> T {
	let ret = unsafe { sp_std::ptr::read_volatile(&dummy) };
	sp_std::mem::forget(dummy);
	ret
}

#[allow(unused_variables, clippy::let_and_return)]
impl Bencher {
	pub fn whitelist(&mut self, key: Vec<u8>, read: bool, write: bool) {
		#[cfg(not(feature = "std"))]
		crate::bench::whitelist(key, read, write);
	}

	pub fn before_run(&self) {
		#[cfg(not(feature = "std"))]
		{
			frame_benchmarking::benchmarking::commit_db();
			frame_benchmarking::benchmarking::wipe_db();
		}
	}

	pub fn count_clear_prefix(&mut self) {
		#[cfg(not(feature = "std"))]
		crate::bench::count_clear_prefix();
	}

	pub fn bench<T, F>(&mut self, mut inner: F) -> T
	where
		F: FnMut() -> T,
	{
		#[cfg(not(feature = "std"))]
		{
			frame_benchmarking::benchmarking::commit_db();
			frame_benchmarking::benchmarking::reset_read_write_count();
			crate::bench::start_timer();
		}

		let ret = black_box(inner());

		#[cfg(not(feature = "std"))]
		{
			let elapsed = crate::bench::end_timer().saturating_sub(crate::bench::redundant_time());
			self.current.elapses.push(elapsed);

			frame_benchmarking::benchmarking::commit_db();

			// changed keys
			self.current.keys = crate::bench::read_written_keys();
		}

		ret
	}

	pub fn print_warnings(&self, name: &str) {
		#[cfg(not(feature = "std"))]
		crate::bench::print_warnings(name.encode());
	}
}
