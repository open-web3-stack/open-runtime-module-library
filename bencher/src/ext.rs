use core::any::{Any, TypeId};
use hash_db::Hasher;
use sp_externalities::{Extension, ExtensionStore, Externalities};
use sp_state_machine::{Backend, ChangesTrieBlockNumber, Ext};
use sp_std::sync::Arc;
use sp_storage::{ChildInfo, TrackedStorageKey};

use super::tracker::ChangesTracker;

pub struct BenchExt<'a, H, N, B>
where
	H: Hasher,
	B: 'a + Backend<H>,
	N: ChangesTrieBlockNumber,
{
	ext: Ext<'a, H, N, B>,
	tracker: Arc<ChangesTracker>,
}

impl<'a, H, N, B> BenchExt<'a, H, N, B>
where
	H: Hasher,
	B: 'a + Backend<H>,
	N: ChangesTrieBlockNumber,
{
	pub fn new(ext: Ext<'a, H, N, B>, tracker: Arc<ChangesTracker>) -> Self {
		BenchExt { ext, tracker }
	}
}

impl<'a, H, N, B> Externalities for BenchExt<'a, H, N, B>
where
	H: Hasher,
	B: 'a + Backend<H>,
	H::Out: Ord + 'static + codec::Codec,
	N: ChangesTrieBlockNumber,
{
	fn set_offchain_storage(&mut self, key: &[u8], value: Option<&[u8]>) {
		self.ext.set_offchain_storage(key, value);
	}

	fn storage(&self, key: &[u8]) -> Option<Vec<u8>> {
		self.ext.storage(key)
	}

	fn storage_hash(&self, key: &[u8]) -> Option<Vec<u8>> {
		self.ext.storage_hash(key)
	}

	fn child_storage_hash(&self, child_info: &ChildInfo, key: &[u8]) -> Option<Vec<u8>> {
		self.ext.child_storage_hash(child_info, key)
	}

	fn child_storage(&self, child_info: &ChildInfo, key: &[u8]) -> Option<Vec<u8>> {
		self.ext.child_storage(child_info, key)
	}

	fn next_storage_key(&self, key: &[u8]) -> Option<Vec<u8>> {
		self.ext.next_storage_key(key)
	}

	fn next_child_storage_key(&self, child_info: &ChildInfo, key: &[u8]) -> Option<Vec<u8>> {
		self.ext.next_child_storage_key(child_info, key)
	}

	fn kill_child_storage(&mut self, child_info: &ChildInfo, limit: Option<u32>) -> (bool, u32) {
		let (all_deleted, num_deleted) = self.ext.kill_child_storage(child_info, limit);
		self.tracker.add_deleted(num_deleted);
		(all_deleted, num_deleted)
	}

	fn clear_prefix(&mut self, prefix: &[u8], limit: Option<u32>) -> (bool, u32) {
		let (all_deleted, num_deleted) = self.ext.clear_prefix(prefix, limit);
		self.tracker.add_deleted(num_deleted);
		(all_deleted, num_deleted)
	}

	fn clear_child_prefix(&mut self, child_info: &ChildInfo, prefix: &[u8], limit: Option<u32>) -> (bool, u32) {
		let (all_deleted, num_deleted) = self.ext.clear_child_prefix(child_info, prefix, limit);
		self.tracker.add_deleted(num_deleted);
		(all_deleted, num_deleted)
	}

	fn place_storage(&mut self, key: Vec<u8>, value: Option<Vec<u8>>) {
		self.tracker.add_main_change(key.clone());
		self.ext.place_storage(key, value);
	}

	fn place_child_storage(&mut self, child_info: &ChildInfo, key: Vec<u8>, value: Option<Vec<u8>>) {
		self.tracker.add_child_change(child_info, key.clone());
		self.ext.place_child_storage(child_info, key, value);
	}

	fn storage_root(&mut self) -> Vec<u8> {
		self.ext.storage_root()
	}

	fn child_storage_root(&mut self, child_info: &ChildInfo) -> Vec<u8> {
		self.ext.child_storage_root(child_info)
	}

	fn storage_append(&mut self, key: Vec<u8>, value: Vec<u8>) {
		self.tracker.add_main_change(key.clone());
		self.ext.storage_append(key, value);
	}

	fn storage_changes_root(&mut self, parent: &[u8]) -> Result<Option<Vec<u8>>, ()> {
		self.ext.storage_changes_root(parent)
	}

	fn storage_start_transaction(&mut self) {
		self.ext.storage_start_transaction();
	}

	fn storage_rollback_transaction(&mut self) -> Result<(), ()> {
		self.ext.storage_rollback_transaction()
	}

	fn storage_commit_transaction(&mut self) -> Result<(), ()> {
		self.ext.storage_commit_transaction()
	}

	fn storage_index_transaction(&mut self, index: u32, hash: &[u8], size: u32) {
		self.ext.storage_index_transaction(index, hash, size);
	}

	fn storage_renew_transaction_index(&mut self, index: u32, hash: &[u8]) {
		self.ext.storage_renew_transaction_index(index, hash);
	}

	fn wipe(&mut self) {
		self.ext.wipe();
	}

	fn commit(&mut self) {
		self.ext.commit();
	}

	fn read_write_count(&self) -> (u32, u32, u32, u32) {
		self.ext.read_write_count()
	}

	fn reset_read_write_count(&mut self) {
		self.tracker.reset();
		self.ext.reset_read_write_count()
	}

	fn get_whitelist(&self) -> Vec<TrackedStorageKey> {
		self.ext.get_whitelist()
	}

	fn set_whitelist(&mut self, new: Vec<TrackedStorageKey>) {
		self.ext.set_whitelist(new);
	}

	fn proof_size(&self) -> Option<u32> {
		self.ext.proof_size()
	}
}

impl<'a, H, N, B> ExtensionStore for BenchExt<'a, H, N, B>
where
	H: Hasher,
	B: 'a + Backend<H>,
	N: ChangesTrieBlockNumber,
{
	fn extension_by_type_id(&mut self, type_id: TypeId) -> Option<&mut dyn Any> {
		self.ext.extension_by_type_id(type_id)
	}

	fn register_extension_with_type_id(
		&mut self,
		type_id: TypeId,
		extension: Box<dyn Extension>,
	) -> Result<(), sp_externalities::Error> {
		self.ext.register_extension_with_type_id(type_id, extension)
	}

	fn deregister_extension_by_type_id(&mut self, type_id: TypeId) -> Result<(), sp_externalities::Error> {
		self.ext.deregister_extension_by_type_id(type_id)
	}
}
