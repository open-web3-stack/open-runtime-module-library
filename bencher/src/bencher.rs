use codec::{Decode, Encode};
use sp_std::prelude::{Box, Vec};

#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct BenchResult {
	pub method: Vec<u8>,
	pub elapses: Vec<u128>,
	pub reads: u32,
	pub writes: u32,
}

pub struct Bencher {
	pub name: Vec<u8>,
	pub results: Vec<BenchResult>,
	pub prepare: Box<dyn Fn()>,
	pub bench: Box<dyn Fn()>,
	pub verify: Box<dyn Fn()>,
}

impl Default for Bencher {
	fn default() -> Self {
		Bencher {
			name: Vec::new(),
			results: Vec::new(),
			prepare: Box::new(|| {}),
			bench: Box::new(|| {}),
			verify: Box::new(|| {}),
		}
	}
}

impl Bencher {
	/// Reset name and blocks
	pub fn reset(&mut self) {
		self.name = Vec::new();
		self.prepare = Box::new(|| {});
		self.bench = Box::new(|| {});
		self.verify = Box::new(|| {});
	}

	/// Set bench name
	pub fn name(&mut self, name: &str) -> &mut Self {
		self.name = name.as_bytes().to_vec();
		self
	}

	/// Set prepare block
	pub fn prepare(&mut self, prepare: impl Fn() + 'static) -> &mut Self {
		self.prepare = Box::new(prepare);
		self
	}

	/// Set verify block
	pub fn verify(&mut self, verify: impl Fn() + 'static) -> &mut Self {
		self.verify = Box::new(verify);
		self
	}

	/// Set bench block
	pub fn bench(&mut self, bench: impl Fn() + 'static) -> &mut Self {
		self.bench = Box::new(bench);
		self
	}

	/// Run benchmark for tests
	#[cfg(feature = "std")]
	pub fn run(&mut self) {
		// Execute prepare block
		(self.prepare)();
		// Execute bench block
		(self.bench)();
		// Execute verify block
		(self.verify)();
	}

	/// Run benchmark
	#[cfg(not(feature = "std"))]
	pub fn run(&mut self) {
		assert!(self.name.len() > 0, "bench name not defined");
		// Warm up the DB
		frame_benchmarking::benchmarking::commit_db();
		frame_benchmarking::benchmarking::wipe_db();

		let mut result = BenchResult {
			method: self.name.clone(),
			..Default::default()
		};

		for _ in 0..50 {
			// Execute prepare block
			(self.prepare)();

			frame_benchmarking::benchmarking::commit_db();
			frame_benchmarking::benchmarking::reset_read_write_count();
			crate::bench::reset();

			let start_time = frame_benchmarking::benchmarking::current_time();
			// Execute bench block
			(self.bench)();
			let end_time = frame_benchmarking::benchmarking::current_time();
			frame_benchmarking::benchmarking::commit_db();

			let (elapsed, reads, writes) = crate::bench::finalized_results(end_time - start_time);

			// Execute verify block
			(self.verify)();

			// Reset the DB
			frame_benchmarking::benchmarking::wipe_db();

			result.elapses.push(elapsed);

			result.reads = sp_std::cmp::max(result.reads, reads);
			result.writes = sp_std::cmp::max(result.writes, writes);
		}
		self.results.push(result);
	}
}
