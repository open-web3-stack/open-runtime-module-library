#![cfg_attr(not(feature = "std"), no_std)]

#[doc(hidden)]
pub extern crate frame_benchmarking;
#[doc(hidden)]
pub extern crate sp_core;
#[doc(hidden)]
pub extern crate sp_std;

use codec::{Decode, Encode};
use sp_std::prelude::Vec;

#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct BenchResult {
	pub method: Vec<u8>,
	pub elapses: Vec<u128>,
	pub reads: u32,
	pub repeat_reads: u32,
	pub writes: u32,
	pub repeat_writes: u32,
}

mod macros;

#[cfg(feature = "std")]
pub mod bench_runner;
#[cfg(feature = "std")]
pub mod handler;

#[cfg(feature = "std")]
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
struct RedundantResult {
	pub identifier: Vec<u8>,
	pub timestamp: u128,
	pub reads: u32,
	pub repeat_reads: u32,
	pub writes: u32,
	pub repeat_writes: u32,
}

#[cfg(feature = "std")]
use rand::{distributions::Alphanumeric, thread_rng, Rng};

#[cfg(feature = "std")]
#[derive(Default)]
/// RedundantMeter is used to measure resources been used by methods that
/// already been benchmarked and have `[orml_weight_meter::weight(..)] macro
/// defined. First method with that macro will be skipped and after that every
/// method with macro defined will be measured as redundant result.
struct RedundantMeter {
	pub started: bool,
	pub results: Vec<RedundantResult>,
	pub current: Option<RedundantResult>,
}

#[cfg(feature = "std")]
impl RedundantMeter {
	/// Entering method with `[orml_weight_meter::weight(..)]` macro
	fn entering_method(&mut self) -> Vec<u8> {
		if !self.started {
			self.started = true;
			return Vec::new();
		}

		if self.current.is_some() {
			return Vec::new();
		}

		let timestamp = frame_benchmarking::benchmarking::current_time();
		let (reads, repeat_reads, writes, repeat_writes) = frame_benchmarking::benchmarking::read_write_count();

		let identifier: Vec<u8> = thread_rng()
			.sample_iter(&Alphanumeric)
			.take(10)
			.map(char::from)
			.collect::<String>()
			.encode();

		self.current = Some(RedundantResult {
			identifier: identifier.clone(),
			timestamp,
			reads,
			repeat_reads,
			writes,
			repeat_writes,
		});

		identifier
	}

	/// Leaving method with `[orml_weight_meter::weight(..)]` macro
	fn leaving_method(&mut self, identifier: &Vec<u8>) {
		if let Some(current) = &self.current {
			if current.identifier.eq(identifier) {
				let (reads, repeat_reads, writes, repeat_writes) = frame_benchmarking::benchmarking::read_write_count();
				let timestamp = frame_benchmarking::benchmarking::current_time();

				self.results.push(RedundantResult {
					identifier: identifier.clone(),
					timestamp: timestamp - current.timestamp,
					reads: reads - current.reads,
					repeat_reads: repeat_reads - current.repeat_reads,
					writes: writes - current.writes,
					repeat_writes: repeat_writes - current.repeat_writes,
				});

				// reset current
				self.current = None;
			}
		}
	}

	/// Take bench results and reset for next measurement
	fn take_results(&mut self) -> (u128, u32, u32, u32, u32) {
		assert!(self.current == None, "benchmark in progress");

		let mut elapsed = 0u128;
		let mut reads = 0u32;
		let mut repeat_reads = 0u32;
		let mut writes = 0u32;
		let mut repeat_writes = 0u32;

		self.results.iter().for_each(|x| {
			elapsed += x.timestamp;
			reads += x.reads;
			repeat_reads += x.repeat_reads;
			writes += x.writes;
			repeat_writes += x.repeat_writes;
		});

		// reset all
		self.started = false;
		self.results = Vec::new();
		self.current = None;

		(elapsed, reads, repeat_reads, writes, repeat_writes)
	}
}

#[cfg(feature = "std")]
thread_local! {
	static REDUNDANT_METER: std::cell::RefCell<RedundantMeter> = std::cell::RefCell::new(RedundantMeter::default());
}

#[sp_runtime_interface::runtime_interface]
pub trait Bencher {
	fn entering_method() -> Vec<u8> {
		REDUNDANT_METER.with(|x| x.borrow_mut().entering_method())
	}

	fn leaving_method(identifier: &Vec<u8>) {
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
}
