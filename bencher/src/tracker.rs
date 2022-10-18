use codec::Encode;
use parking_lot::RwLock;
use sp_state_machine::StorageKey;
use sp_storage::ChildInfo;
use std::{collections::HashMap, sync::Arc, time::Instant};

#[derive(PartialEq, Eq)]
enum AccessType {
	None,
	Redundant,
	Important,
	Whitelisted,
}

impl Default for AccessType {
	fn default() -> Self {
		AccessType::None
	}
}

impl AccessType {
	fn is_important(&self) -> bool {
		*self == AccessType::Important
	}
	fn mark_important(&mut self) {
		if *self != AccessType::Whitelisted {
			*self = AccessType::Important;
		}
	}
}

#[derive(Default)]
struct AccessInfo {
	pub read: AccessType,
	pub written: AccessType,
}

impl AccessInfo {
	fn read(redundant: bool) -> Self {
		let read = if redundant {
			AccessType::Redundant
		} else {
			AccessType::Important
		};
		Self {
			read,
			written: AccessType::None,
		}
	}

	fn written(redundant: bool) -> Self {
		let written = if redundant {
			AccessType::Redundant
		} else {
			AccessType::Important
		};
		Self {
			read: AccessType::Redundant,
			written,
		}
	}

	fn whitelisted(read: bool, write: bool) -> Self {
		Self {
			read: if read {
				AccessType::Whitelisted
			} else {
				AccessType::None
			},
			written: if write {
				AccessType::Whitelisted
			} else {
				AccessType::None
			},
		}
	}
}

#[derive(Default, Debug)]
struct AccessReport {
	pub read: u32,
	pub written: u32,
}

#[derive(thiserror::Error, Copy, Clone, Debug)]
pub enum Warning {
	#[error("clear prefix without limit, cannot be tracked")]
	ClearPrefixWithoutLimit,
	#[error("child storage is not supported")]
	ChildStorageNotSupported,
}

pub struct BenchTracker {
	instant: RwLock<Instant>,
	depth: RwLock<u32>,
	redundant: RwLock<Instant>,
	results: RwLock<Vec<u128>>,
	main_keys: RwLock<HashMap<StorageKey, AccessInfo>>,
	clear_prefixes: RwLock<HashMap<StorageKey, u32>>,
	warnings: RwLock<Vec<Warning>>,
	whitelisted_keys: RwLock<HashMap<StorageKey, (bool, bool)>>,
	count_clear_prefix: RwLock<bool>,
}

impl BenchTracker {
	pub fn new() -> Self {
		BenchTracker {
			instant: RwLock::new(Instant::now()),
			depth: RwLock::new(0),
			redundant: RwLock::new(Instant::now()),
			results: RwLock::new(Vec::new()),
			main_keys: RwLock::new(HashMap::new()),
			clear_prefixes: RwLock::new(HashMap::new()),
			warnings: RwLock::new(Vec::new()),
			whitelisted_keys: RwLock::new(HashMap::new()),
			count_clear_prefix: RwLock::new(false),
		}
	}

	pub fn warnings(&self) -> Vec<Warning> {
		let warnings = &*self.warnings.read();
		warnings.clone()
	}

	pub fn instant(&self) {
		*self.instant.write() = Instant::now();
	}

	pub fn elapsed(&self) -> u128 {
		self.instant.read().elapsed().as_nanos()
	}

	pub fn is_redundant(&self) -> bool {
		*self.depth.read() > 1
	}

	pub fn on_read_storage(&self, key: StorageKey) {
		let redundant = self.is_redundant();
		let main_keys = &mut *self.main_keys.write();
		match main_keys.get_mut(&key) {
			Some(info) => {
				if redundant {
					return;
				}
				if info.written.is_important() {
					return;
				}
				info.read.mark_important();
			}
			None => {
				main_keys.insert(key, AccessInfo::read(redundant));
			}
		};
	}

	pub fn on_read_child_storage(&self, _child_info: &ChildInfo, _key: StorageKey) {
		if self.is_redundant() {
			return;
		}
		self.warn(Warning::ChildStorageNotSupported);
	}

	pub fn on_update_storage(&self, key: StorageKey) {
		let redundant = self.is_redundant();
		let main_keys = &mut *self.main_keys.write();
		match main_keys.get_mut(&key) {
			Some(info) => {
				if redundant {
					return;
				}
				info.written.mark_important();
			}
			None => {
				main_keys.insert(key, AccessInfo::written(redundant));
			}
		};
	}

	pub fn on_update_child_storage(&self, _child_info: &ChildInfo, _key: StorageKey) {
		if self.is_redundant() {
			return;
		}
		self.warn(Warning::ChildStorageNotSupported);
	}

	pub fn on_clear_prefix(&self, prefix: &[u8], limit: Option<u32>) {
		if self.is_redundant() || !(*self.count_clear_prefix.read()) {
			return;
		}
		if let Some(limit) = limit {
			let key = prefix.to_vec();
			let clear_prefixes = &mut *self.clear_prefixes.write();
			match clear_prefixes.get_mut(&key) {
				Some(n) => {
					*n += limit;
				}
				None => {
					clear_prefixes.insert(key, limit);
				}
			};
		} else {
			self.warn(Warning::ClearPrefixWithoutLimit);
		}
	}

	pub fn on_clear_child_prefix(&self, _child_info: &ChildInfo, _prefix: &[u8], _limit: Option<u32>) {
		if self.is_redundant() {
			return;
		}
		self.warn(Warning::ChildStorageNotSupported);
	}

	pub fn on_kill_child_storage(&self, _child_info: &ChildInfo, _limit: Option<u32>) {
		if self.is_redundant() {
			return;
		}
		self.warn(Warning::ChildStorageNotSupported);
	}

	/// Get the benchmark summary
	pub fn read_written_keys(&self) -> Vec<u8> {
		let mut summary = HashMap::<StorageKey, AccessReport>::new();

		self.main_keys.read().iter().for_each(|(key, info)| {
			let prefix_end = core::cmp::min(32, key.len());
			let prefix = key[0..prefix_end].to_vec();
			if let Some(report) = summary.get_mut(&prefix) {
				if info.read.is_important() {
					report.read += 1;
				}
				if info.written.is_important() {
					report.written += 1;
				}
			} else {
				let mut report = AccessReport::default();
				if info.read.is_important() {
					report.read += 1;
				}
				if info.written.is_important() {
					report.written += 1;
				}
				if report.read + report.written > 0 {
					summary.insert(prefix, report);
				}
			}
		});

		self.clear_prefixes.read().iter().for_each(|(key, items)| {
			let prefix_end = core::cmp::min(32, key.len());
			let prefix = key[0..prefix_end].to_vec();
			if let Some(report) = summary.get_mut(&prefix) {
				report.written += items;
			} else {
				summary.insert(
					prefix,
					AccessReport {
						written: *items,
						..Default::default()
					},
				);
			}
		});

		summary
			.into_iter()
			.map(|(prefix, report)| (prefix, report.read, report.written))
			.collect::<Vec<(StorageKey, u32, u32)>>()
			.encode()
	}

	/// Run before executing the code been benchmarked
	pub fn before_block(&self) {
		let timestamp = Instant::now();

		let mut depth = self.depth.write();

		if *depth == 0 {
			*depth = 1;
			return;
		}

		if *depth == 1 {
			*self.redundant.write() = timestamp;
		}

		*depth += 1;
	}

	/// Run after benchmarking code is executed
	pub fn after_block(&self) {
		let mut depth = self.depth.write();
		if *depth == 2 {
			let redundant = self.redundant.read();
			let elapsed = redundant.elapsed().as_nanos();
			self.results.write().push(elapsed);
		}
		*depth -= 1;
	}

	/// Add a warning to be printed after execution
	pub fn warn(&self, warning: Warning) {
		let mut warnings = self.warnings.write();
		warnings.push(warning);
	}

	/// Redundant elapsed time
	pub fn redundant_time(&self) -> u128 {
		assert_eq!(*self.depth.read(), 0, "benchmark in progress");

		let mut elapsed = 0u128;

		self.results.read().iter().for_each(|x| {
			elapsed = elapsed.saturating_add(*x);
		});

		elapsed
	}

	/// Prepare tracker for next run
	pub fn prepare_next_run(&self) {
		*self.depth.write() = 0;
		self.results.write().clear();

		self.clear_prefixes.write().clear();
		self.warnings.write().clear();

		let main_keys = &mut self.main_keys.write();
		main_keys.clear();

		let keys = self.whitelisted_keys.read();
		for (key, (read, write)) in keys.iter() {
			main_keys.insert(key.clone(), AccessInfo::whitelisted(*read, *write));
		}
	}

	/// Whitelist keys that don't need to be tracked
	pub fn whitelist(&self, key: Vec<u8>, read: bool, write: bool) {
		let whitelisted = &mut self.whitelisted_keys.write();
		whitelisted.insert(key, (read, write));
	}

	pub fn count_clear_prefix(&self) {
		*self.count_clear_prefix.write() = true;
	}

	/// Reset for the next benchmark
	pub fn reset(&self) {
		*self.depth.write() = 0;
		*self.redundant.write() = Instant::now();
		self.results.write().clear();
		self.main_keys.write().clear();
		self.clear_prefixes.write().clear();
		self.warnings.write().clear();
		self.whitelisted_keys.write().clear();
		*self.count_clear_prefix.write() = false;
	}
}

sp_externalities::decl_extension! {
	pub struct BenchTrackerExt(Arc<BenchTracker>);
}
