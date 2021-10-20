use codec::{Decode, Encode};
use sp_std::prelude::Vec;

#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct BenchResult {
	pub method: Vec<u8>,
	pub elapses: Vec<u128>,
	pub reads: u32,
	pub writes: u32,
	pub keys: Vec<u8>,
}

#[derive(Default)]
pub struct Bencher {
	pub name: Vec<u8>,
	pub results: Vec<BenchResult>,
}

pub fn black_box<T>(dummy: T) -> T {
	unsafe {
		let ret = sp_std::ptr::read_volatile(&dummy);
		sp_std::mem::forget(dummy);
		ret
	}
}

impl Bencher {
	/// Reset name and blocks
	pub fn reset(&mut self) {
		self.name = Vec::new();
	}

	/// Set bench name
	pub fn name(&mut self, name: &str) -> &mut Self {
		self.name = name.as_bytes().to_vec();
		self
	}

	#[cfg(feature = "std")]
	pub fn bench<T, F>(&mut self, mut inner: F) -> T
	where
		F: FnMut() -> T,
	{
		black_box(inner())
	}

	#[cfg(not(feature = "std"))]
	pub fn bench<T, F>(&mut self, mut inner: F) -> T
	where
		F: FnMut() -> T,
	{
		frame_benchmarking::benchmarking::commit_db();
		frame_benchmarking::benchmarking::reset_read_write_count();
		crate::bench::reset_redundant();

		let mut result = self.results.pop().unwrap();
		crate::bench::instant();
		let ret = black_box(inner());
		let elapsed = crate::bench::elapsed().saturating_sub(crate::bench::redundant_time());
		result.elapses.push(elapsed);

		frame_benchmarking::benchmarking::commit_db();
		let (reads, _, written, _) = frame_benchmarking::benchmarking::read_write_count();

		result.reads = reads;
		result.writes = written;
		// changed keys
		result.keys = crate::bench::read_written_keys();
		self.results.push(result);
		ret
	}

	pub fn print_warnings(&self) {
		crate::bench::print_warnings(self.name.clone().encode());
	}
}
