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

pub struct BenchTracker {
	instant: RwLock<Instant>,
	depth: RwLock<u32>,
	redundant: RwLock<Instant>,
	results: RwLock<Vec<u128>>,
	main_keys: RwLock<HashMap<StorageKey, AccessInfo>>,
	child_keys: RwLock<HashMap<StorageKey, HashMap<StorageKey, AccessInfo>>>,
	warn_child_prefix_remove: RwLock<bool>,
	whitelisted_keys: RwLock<HashMap<StorageKey, (bool, bool)>>,
}

impl BenchTracker {
	pub fn new() -> Self {
		BenchTracker {
			instant: RwLock::new(Instant::now()),
			depth: RwLock::new(0),
			redundant: RwLock::new(Instant::now()),
			results: RwLock::new(Vec::new()),
			main_keys: RwLock::new(HashMap::new()),
			child_keys: RwLock::new(HashMap::new()),
			warn_child_prefix_remove: RwLock::new(false),
			whitelisted_keys: RwLock::new(HashMap::new()),
		}
	}

	pub fn has_warn_child_prefix_removal(&self) -> bool {
		*self.warn_child_prefix_remove.read()
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

	pub fn reading_key(&self, key: StorageKey) {
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

	pub fn reading_child_key(&self, child_info: &ChildInfo, key: StorageKey) {
		let redundant = self.is_redundant();
		let child_keys = &mut *self.child_keys.write();
		let storage_key = child_info.storage_key().to_vec();
		match child_keys.get_mut(&storage_key) {
			Some(reads) => {
				match reads.get_mut(&key) {
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
						reads.insert(key, AccessInfo::read(redundant));
					}
				};
			}
			None => {
				let mut reads = HashMap::<StorageKey, AccessInfo>::new();
				reads.insert(key, AccessInfo::read(redundant));
				child_keys.insert(storage_key, reads);
			}
		};
	}

	pub fn changing_key(&self, key: StorageKey) {
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

	pub fn changing_child_key(&self, child_info: &ChildInfo, key: StorageKey) {
		let redundant = self.is_redundant();
		let child_keys = &mut *self.child_keys.write();
		let storage_key = child_info.storage_key().to_vec();
		match child_keys.get_mut(&storage_key) {
			Some(changes) => {
				match changes.get_mut(&key) {
					Some(info) => {
						if redundant {
							return;
						}
						info.written.mark_important();
					}
					None => {
						changes.insert(key, AccessInfo::written(redundant));
					}
				};
			}
			None => {
				let mut changes = HashMap::<StorageKey, AccessInfo>::new();
				changes.insert(key, AccessInfo::written(redundant));
				child_keys.insert(storage_key, changes);
			}
		};
	}

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

		self.child_keys.read().iter().for_each(|(prefix, keys)| {
			keys.iter().for_each(|(key, info)| {
				let prefix_end = core::cmp::min(32, prefix.len() + key.len());
				let prefix = [prefix.clone(), key.clone()].concat()[0..prefix_end].to_vec();
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
		});

		summary
			.into_iter()
			.map(|(prefix, report)| (prefix, report.read, report.written))
			.collect::<Vec<(StorageKey, u32, u32)>>()
			.encode()
	}

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

	pub fn after_block(&self) {
		let mut depth = self.depth.write();
		if *depth == 2 {
			let redundant = self.redundant.read();
			let elapsed = redundant.elapsed().as_nanos();
			self.results.write().push(elapsed);
		}
		*depth -= 1;
	}

	pub fn warn_child_prefix_removal(&self) {
		*self.warn_child_prefix_remove.write() = true;
	}

	pub fn redundant_time(&self) -> u128 {
		assert_eq!(*self.depth.read(), 0, "benchmark in progress");

		let mut elapsed = 0u128;

		self.results.read().iter().for_each(|x| {
			elapsed = elapsed.saturating_add(*x);
		});

		elapsed
	}

	pub fn prepare(&self) {
		*self.depth.write() = 0;
		self.results.write().clear();

		self.child_keys.write().clear();
		*self.warn_child_prefix_remove.write() = false;

		let main_keys = &mut self.main_keys.write();
		main_keys.clear();

		let keys = self.whitelisted_keys.read();
		for (key, (read, write)) in keys.iter() {
			main_keys.insert(key.clone(), AccessInfo::whitelisted(*read, *write));
		}
	}

	pub fn whitelist(&self, key: Vec<u8>, read: bool, write: bool) {
		let whitelisted = &mut self.whitelisted_keys.write();
		whitelisted.insert(key, (read, write));
	}

	pub fn reset(&self) {
		*self.depth.write() = 0;
		*self.redundant.write() = Instant::now();
		self.results.write().clear();
		self.main_keys.write().clear();
		self.child_keys.write().clear();
		*self.warn_child_prefix_remove.write() = false;
		self.whitelisted_keys.write().clear();
	}
}

sp_externalities::decl_extension! {
	pub struct BenchTrackerExt(Arc<BenchTracker>);
}
