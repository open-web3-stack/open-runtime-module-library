use codec::{Decode, Encode};
use rstd::{iter, marker, prelude::*};
use sr_primitives::{traits::Member, RuntimeDebug};
use support::{Parameter, StorageMap};

#[derive(RuntimeDebug, PartialEq, Eq, Encode, Decode)]
pub struct LinkedItem<Item> {
	pub prev: Option<Item>,
	pub next: Option<Item>,
}

impl<Item> Default for LinkedItem<Item> {
	fn default() -> Self {
		LinkedItem { prev: None, next: None }
	}
}

pub struct LinkedList<Storage, Key, Item>(rstd::marker::PhantomData<(Storage, Key, Item)>);

impl<Storage, Key, Value> LinkedList<Storage, Key, Value>
where
	Value: Parameter + Member + Copy,
	Key: Parameter,
	Storage: 'static + StorageMap<(Key, Option<Value>), LinkedItem<Value>, Query = Option<LinkedItem<Value>>>,
{
	fn read_head(key: &Key) -> LinkedItem<Value> {
		Self::read(key, None)
	}

	fn write_head(key: &Key, item: LinkedItem<Value>) {
		Self::write(key, None, item);
	}

	fn read(key: &Key, value: Option<Value>) -> LinkedItem<Value> {
		Storage::get(&(key.clone(), value)).unwrap_or_else(|| Default::default())
	}

	fn take(key: &Key, value: Value) -> LinkedItem<Value> {
		let item = Self::read(key, Some(value));
		Self::remove(key, value);
		item
	}

	fn write(key: &Key, value: Option<Value>, item: LinkedItem<Value>) {
		Storage::insert(&(key.clone(), value), item);
	}

	pub fn append(key: &Key, value: Value) {
		let head = Self::read_head(key);
		let new_head = LinkedItem {
			prev: Some(value),
			next: head.next,
		};

		Self::write_head(key, new_head);

		let prev = Self::read(key, head.prev);
		let new_prev = LinkedItem {
			prev: prev.prev,
			next: Some(value),
		};
		Self::write(key, head.prev, new_prev);

		let item = LinkedItem {
			prev: head.prev,
			next: None,
		};
		Self::write(key, Some(value), item);
	}

	pub fn remove(key: &Key, value: Value) {
		if let Some(item) = Storage::take(&(key.clone(), Some(value))) {
			let prev = Self::read(key, item.prev);
			let new_prev = LinkedItem {
				prev: prev.prev,
				next: item.next,
			};

			Self::write(key, item.prev, new_prev);

			let next = Self::read(key, item.next);
			let new_next = LinkedItem {
				prev: item.prev,
				next: next.next,
			};

			Self::write(key, item.next, new_next);
		}
	}

	pub fn enumerate(key: &Key) -> Enumerator<Key, Value, Self> {
		Enumerator::<Key, Value, Self>::new(key, false, Self::read_head(key))
	}

	pub fn take_all(key: &Key) -> Enumerator<Key, Value, Self> {
		Enumerator::<Key, Value, Self>::new(key, true, Self::read_head(key))
	}
}

pub struct Enumerator<Key, Value, LinkedList> {
	key: Key,
	should_take: bool,
	linkage: LinkedItem<Value>,
	next_fn: fn(&mut Enumerator<Key, Value, LinkedList>) -> Option<Value>,
	_phantom: marker::PhantomData<LinkedList>,
}

impl<Key, Value, Storage> Enumerator<Key, Value, LinkedList<Storage, Key, Value>>
where
	Key: Parameter,
	Value: Parameter + Member + Copy,
	Storage: 'static + StorageMap<(Key, Option<Value>), LinkedItem<Value>, Query = Option<LinkedItem<Value>>>,
{
	fn new(key: &Key, should_take: bool, linkage: LinkedItem<Value>) -> Self {
		Self {
			key: key.clone(),
			should_take,
			linkage,
			next_fn: Self::next,
			_phantom: Default::default(),
		}
	}
	fn next(&mut self) -> Option<Value> {
		let next_value = self.linkage.next?;
		if self.should_take {
			self.linkage = <LinkedList<Storage, Key, Value>>::take(&self.key, next_value);
		} else {
			self.linkage = <LinkedList<Storage, Key, Value>>::read(&self.key, Some(next_value));
		}
		Some(next_value)
	}
}

impl<Key, Value, Storage> iter::Iterator for Enumerator<Key, Value, LinkedList<Storage, Key, Value>>
where
	Key: Parameter,
	Value: Parameter + Member + Copy,
	Storage: 'static + StorageMap<(Key, Option<Value>), LinkedItem<Value>, Query = Option<LinkedItem<Value>>>,
{
	type Item = Value;

	fn next(&mut self) -> Option<Self::Item> {
		let next_fn = self.next_fn;
		next_fn(self)
	}
}

impl<Key, Value, LinkedList> Drop for Enumerator<Key, Value, LinkedList> {
	fn drop(&mut self) {
		if !self.should_take {
			return;
		}

		let next_fn = self.next_fn;
		while next_fn(self).is_some() {}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use primitives::H256;
	use sr_primitives::{testing::Header, traits::IdentityLookup, Perbill};
	use support::{decl_module, decl_storage, impl_outer_origin, parameter_types, StorageMap};

	type Key = u64;
	type Value = u32;
	pub trait Trait: system::Trait {}

	type TestLinkedItem = LinkedItem<Value>;

	decl_storage! {
		trait Store for Module<T: Trait> as Test {
			pub TestItem get(linked_list): map (Key, Option<Value>) => Option<TestLinkedItem>;
		}
	}

	decl_module! {
		pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		}
	}

	impl_outer_origin! {
		pub enum Origin for Test {}
	}

	// For testing the module, we construct most of a mock runtime. This means
	// first constructing a configuration type (`Test`) which `impl`s each of the
	// configuration traits of modules we want to use.
	#[derive(Clone, Eq, PartialEq, Debug)]
	pub struct Test;
	parameter_types! {
		pub const BlockHashCount: u64 = 250;
		pub const MaximumBlockWeight: u32 = 1024;
		pub const MaximumBlockLength: u32 = 2 * 1024;
		pub const AvailableBlockRatio: Perbill = Perbill::one();
	}
	impl system::Trait for Test {
		type Origin = Origin;
		type Index = u64;
		type BlockNumber = u64;
		type Call = ();
		type Hash = H256;
		type Hashing = ::sr_primitives::traits::BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = ();
		type BlockHashCount = BlockHashCount;
		type MaximumBlockWeight = MaximumBlockWeight;
		type MaximumBlockLength = MaximumBlockLength;
		type AvailableBlockRatio = AvailableBlockRatio;
		type Version = ();
	}

	type TestLinkedList = LinkedList<TestItem, Key, Value>;

	pub fn new_test_ext() -> runtime_io::TestExternalities {
		system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
	}

	#[test]
	fn linked_list_can_append_values() {
		new_test_ext().execute_with(|| {
			TestLinkedList::append(&0, 1);

			assert_eq!(
				TestItem::get(&(0, None)),
				Some(TestLinkedItem {
					prev: Some(1),
					next: Some(1),
				})
			);

			assert_eq!(TestItem::get(&(0, Some(1))), Some(Default::default()));

			TestLinkedList::append(&0, 2);

			assert_eq!(
				TestItem::get(&(0, None)),
				Some(TestLinkedItem {
					prev: Some(2),
					next: Some(1),
				})
			);

			assert_eq!(
				TestItem::get(&(0, Some(1))),
				Some(TestLinkedItem {
					prev: None,
					next: Some(2),
				})
			);

			assert_eq!(
				TestItem::get(&(0, Some(2))),
				Some(TestLinkedItem {
					prev: Some(1),
					next: None,
				})
			);

			TestLinkedList::append(&0, 3);

			assert_eq!(
				TestItem::get(&(0, None)),
				Some(TestLinkedItem {
					prev: Some(3),
					next: Some(1),
				})
			);

			assert_eq!(
				TestItem::get(&(0, Some(1))),
				Some(TestLinkedItem {
					prev: None,
					next: Some(2),
				})
			);

			assert_eq!(
				TestItem::get(&(0, Some(2))),
				Some(TestLinkedItem {
					prev: Some(1),
					next: Some(3),
				})
			);

			assert_eq!(
				TestItem::get(&(0, Some(3))),
				Some(TestLinkedItem {
					prev: Some(2),
					next: None,
				})
			);
		});
	}

	#[test]
	fn linked_list_can_remove_values() {
		new_test_ext().execute_with(|| {
			TestLinkedList::append(&0, 1);
			TestLinkedList::append(&0, 2);
			TestLinkedList::append(&0, 3);

			TestLinkedList::remove(&0, 2);

			assert_eq!(
				TestItem::get(&(0, None)),
				Some(TestLinkedItem {
					prev: Some(3),
					next: Some(1),
				})
			);

			assert_eq!(
				TestItem::get(&(0, Some(1))),
				Some(TestLinkedItem {
					prev: None,
					next: Some(3),
				})
			);

			assert_eq!(TestItem::get(&(0, Some(2))), None);

			assert_eq!(
				TestItem::get(&(0, Some(3))),
				Some(TestLinkedItem {
					prev: Some(1),
					next: None,
				})
			);

			TestLinkedList::remove(&0, 1);

			assert_eq!(
				TestItem::get(&(0, None)),
				Some(TestLinkedItem {
					prev: Some(3),
					next: Some(3),
				})
			);

			assert_eq!(TestItem::get(&(0, Some(1))), None);

			assert_eq!(TestItem::get(&(0, Some(2))), None);

			assert_eq!(TestItem::get(&(0, Some(3))), Some(Default::default()));

			TestLinkedList::remove(&0, 3);

			assert_eq!(TestItem::get(&(0, None)), Some(Default::default()));

			assert_eq!(TestItem::get(&(0, Some(1))), None);

			assert_eq!(TestItem::get(&(0, Some(2))), None);

			assert_eq!(TestItem::get(&(0, Some(2))), None);
		});
	}

	#[test]
	fn linked_list_can_enumerate() {
		new_test_ext().execute_with(|| {
			assert_eq!(TestLinkedList::enumerate(&0).collect::<Vec<_>>(), []);

			TestLinkedList::append(&0, 1);
			TestLinkedList::append(&0, 2);
			TestLinkedList::append(&0, 3);

			// iteration
			assert_eq!(TestLinkedList::enumerate(&0).collect::<Vec<_>>(), [1, 2, 3]);

			// should not take
			assert_eq!(TestLinkedList::enumerate(&0).collect::<Vec<_>>(), [1, 2, 3]);
		});
	}

	#[test]
	fn linked_list_can_take_all() {
		new_test_ext().execute_with(|| {
			assert_eq!(TestLinkedList::take_all(&0).collect::<Vec<_>>(), []);

			TestLinkedList::append(&0, 1);
			TestLinkedList::append(&0, 2);
			TestLinkedList::append(&0, 3);

			assert_eq!(TestLinkedList::take_all(&0).collect::<Vec<_>>(), [1, 2, 3]);

			assert_eq!(TestItem::get(&(0, Some(1))), None);
			assert_eq!(TestItem::get(&(0, Some(2))), None);
			assert_eq!(TestItem::get(&(0, Some(3))), None);
			assert_eq!(TestLinkedList::enumerate(&0).collect::<Vec<_>>(), []);
		});
	}

	#[test]
	fn linked_list_take_all_is_safe() {
		new_test_ext().execute_with(|| {
			assert_eq!(TestLinkedList::take_all(&0).collect::<Vec<_>>(), []);

			TestLinkedList::append(&0, 1);
			TestLinkedList::append(&0, 2);
			TestLinkedList::append(&0, 3);

			let _ = TestLinkedList::take_all(&0);

			assert_eq!(TestItem::get(&(0, Some(1))), None);
			assert_eq!(TestItem::get(&(0, Some(2))), None);
			assert_eq!(TestItem::get(&(0, Some(3))), None);
			assert_eq!(TestLinkedList::enumerate(&0).collect::<Vec<_>>(), []);
		});
	}
}
