use parking_lot::RwLock;
use sp_state_machine::StorageKey;
use sp_storage::ChildInfo;
use std::{collections::HashMap, sync::Arc};

#[derive(Default)]
pub struct ChangesTracker {
	deleted: RwLock<u32>,
	main_changes: RwLock<Vec<StorageKey>>,
	child_changes: RwLock<HashMap<StorageKey, Vec<StorageKey>>>,
}

impl ChangesTracker {
	pub fn new() -> Self {
		ChangesTracker::default()
	}

	pub fn add_deleted(&self, num_deleted: u32) {
		*self.deleted.write() += num_deleted;
	}

	pub fn add_main_change(&self, key: StorageKey) {
		let keys = &mut *self.main_changes.write();
		if !keys.contains(&key) {
			keys.push(key);
		}
	}

	pub fn add_child_change(&self, child_info: &ChildInfo, key: StorageKey) {
		let child_changes = &mut *self.child_changes.write();
		let storage_key = child_info.storage_key().to_vec();
		if let Some(keys) = child_changes.get_mut(&storage_key) {
			if !keys.contains(&key) {
				keys.push(key);
			}
		} else {
			child_changes.insert(storage_key, vec![key]);
		}
	}

	pub fn changes_count(&self) -> u32 {
		let mut changes_count = self.main_changes.read().len();
		changes_count += self.child_changes.read().iter().map(|(_, keys)| keys).flatten().count();
		(changes_count as u32) + *self.deleted.read()
	}

	pub fn reset(&self) {
		self.main_changes.write().clear();
		self.child_changes.write().clear();
		*self.deleted.write() = 0;
	}
}

sp_externalities::decl_extension! {
	pub struct ChangesTrackerExt(Arc<ChangesTracker>);
}
