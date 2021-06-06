#![cfg_attr(not(feature = "std"), no_std)]

#[doc(hidden)]
pub extern crate frame_benchmarking;
#[doc(hidden)]
pub extern crate paste;
#[doc(hidden)]
pub extern crate sp_core;
#[doc(hidden)]
pub extern crate sp_io;
#[doc(hidden)]
pub extern crate sp_std;

mod macros;

#[cfg(feature = "std")]
pub mod bench_runner;
#[cfg(feature = "std")]
pub mod build_wasm;
#[cfg(feature = "std")]
mod colorize;
#[cfg(feature = "std")]
pub mod handler;
#[cfg(feature = "std")]
mod redundant_meter;

use codec::{Decode, Encode};
use sp_std::prelude::{Box, Vec};

#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct BenchResult {
	pub method: Vec<u8>,
	pub elapses: Vec<u128>,
	pub reads: u32,
	pub repeat_reads: u32,
	pub writes: u32,
	pub repeat_writes: u32,
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
			bencher::reset();

			let start_time = frame_benchmarking::benchmarking::current_time();
			// Execute bench block
			(self.bench)();
			let end_time = frame_benchmarking::benchmarking::current_time();
			frame_benchmarking::benchmarking::commit_db();

			let (elapsed, reads, repeat_reads, writes, repeat_writes) =
				bencher::finalized_results(end_time - start_time);

			// Execute verify block
			(self.verify)();

			// Reset the DB
			frame_benchmarking::benchmarking::wipe_db();

			result.elapses.push(elapsed);

			result.reads = sp_std::cmp::max(result.reads, reads);
			result.repeat_reads = sp_std::cmp::max(result.repeat_reads, repeat_reads);
			result.writes = sp_std::cmp::max(result.writes, writes);
			result.repeat_writes = sp_std::cmp::max(result.repeat_writes, repeat_writes);
		}
		self.results.push(result);
	}
}

#[cfg(feature = "std")]
thread_local! {
	static REDUNDANT_METER: std::cell::RefCell<redundant_meter::RedundantMeter> = std::cell::RefCell::new(redundant_meter::RedundantMeter::default());
}

#[sp_runtime_interface::runtime_interface]
pub trait Bencher {
	fn panic(str: Vec<u8>) {
		let msg = String::from_utf8_lossy(&str);
		eprintln!("{}", colorize::red_bold(&msg));
		std::process::exit(-1);
	}

	fn entering_method() -> Vec<u8> {
		REDUNDANT_METER.with(|x| x.borrow_mut().entering_method())
	}

	fn leaving_method(identifier: Vec<u8>) {
		REDUNDANT_METER.with(|x| {
			x.borrow_mut().leaving_method(identifier);
		});
	}

	fn finalized_results(elapsed: u128) -> (u128, u32, u32, u32, u32) {
		let (reads, repeat_reads, writes, repeat_writes) = frame_benchmarking::benchmarking::read_write_count();

		let (redundant_elapsed, redundant_reads, redundant_repeat_reads, redundant_writes, redundant_repeat_writes) =
			REDUNDANT_METER.with(|x| x.borrow_mut().take_results());

		let elapsed = elapsed - redundant_elapsed;
		let reads = reads - redundant_reads;
		let repeat_reads = repeat_reads - redundant_repeat_reads;
		let writes = writes - redundant_writes;
		let repeat_writes = repeat_writes - redundant_repeat_writes;

		(elapsed, reads, repeat_reads, writes, repeat_writes)
	}

	fn reset() {
		REDUNDANT_METER.with(|x| {
			x.borrow_mut().reset();
		});
	}
}
