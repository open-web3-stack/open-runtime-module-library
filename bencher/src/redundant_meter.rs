use codec::{Decode, Encode};
use rand::{distributions::Alphanumeric, thread_rng, Rng};

#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
struct RedundantResult {
	identifier: Vec<u8>,
	timestamp: u128,
	reads: u32,
	writes: u32,
}

#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct RedundantOutput {
	pub elapsed: u128,
	pub reads: u32,
	pub writes: u32,
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
		let writes = super::bench::storage_changes_count();
		let (reads, _, _, _) = frame_benchmarking::benchmarking::read_write_count();

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
			writes,
		});

		identifier
	}

	/// Leaving method with `[orml_weight_meter::weight(..)]` macro
	pub fn leaving_method(&mut self, identifier: Vec<u8>) {
		if let Some(current) = &self.current {
			if current.identifier.eq(&identifier) {
				let writes = super::bench::storage_changes_count();
				let (reads, _, _, _) = frame_benchmarking::benchmarking::read_write_count();
				let timestamp = frame_benchmarking::benchmarking::current_time();

				self.results.push(RedundantResult {
					identifier,
					timestamp: timestamp.saturating_sub(current.timestamp),
					reads: reads.saturating_sub(current.reads),
					writes: writes.saturating_sub(current.writes),
				});

				// reset current
				self.current = None;
			}
		}
	}

	/// Take bench results and reset for next measurement
	pub fn take_results(&mut self) -> RedundantOutput {
		assert!(self.current == None, "benchmark in progress");

		let mut elapsed = 0u128;
		let mut reads = 0u32;
		let mut writes = 0u32;

		self.results.iter().for_each(|x| {
			elapsed += x.timestamp;
			reads += x.reads;
			writes += x.writes;
		});

		self.reset();

		RedundantOutput { elapsed, reads, writes }
	}

	pub fn reset(&mut self) {
		self.started = false;
		self.results = Vec::new();
		self.current = None;
	}
}
