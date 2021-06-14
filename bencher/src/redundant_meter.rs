use codec::{Decode, Encode};
use rand::{distributions::Alphanumeric, thread_rng, Rng};

#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
struct RedundantResult {
	identifier: Vec<u8>,
	timestamp: u128,
	reads: u32,
	repeat_reads: u32,
	writes: u32,
	repeat_writes: u32,
}

/// RedundantMeter is used to measure resources been used by methods that
/// already been benchmarked and have `[orml_weight_meter::weight(..)] macro
/// defined. First method with that macro will be skipped and after that every
/// method with macro defined will be measured as redundant result.
#[derive(Default)]
pub struct RedundantMeter {
	started: bool,
	results: Vec<RedundantResult>,
	current: Option<RedundantResult>,
}

impl RedundantMeter {
	/// Entering method with `[orml_weight_meter::weight(..)]` macro
	pub fn entering_method(&mut self) -> Vec<u8> {
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
			identifier: identifier.to_owned(),
			timestamp,
			reads,
			repeat_reads,
			writes,
			repeat_writes,
		});

		identifier
	}

	/// Leaving method with `[orml_weight_meter::weight(..)]` macro
	pub fn leaving_method(&mut self, identifier: Vec<u8>) {
		if let Some(current) = &self.current {
			if current.identifier.eq(&identifier) {
				let (reads, repeat_reads, writes, repeat_writes) = frame_benchmarking::benchmarking::read_write_count();
				let timestamp = frame_benchmarking::benchmarking::current_time();

				self.results.push(RedundantResult {
					identifier,
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
	pub fn take_results(&mut self) -> (u128, u32, u32, u32, u32) {
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

		self.reset();

		(elapsed, reads, repeat_reads, writes, repeat_writes)
	}

	pub fn reset(&mut self) {
		self.started = false;
		self.results = Vec::new();
		self.current = None;
	}
}
