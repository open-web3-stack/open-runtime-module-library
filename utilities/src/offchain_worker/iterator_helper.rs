use super::OffchainErr;
use codec::{Decode, EncodeLike, FullCodec, FullEncode};
use frame_support::{
	storage::{
		generator::{StorageDoubleMap as StorageDoubleMapT, StorageMap as StorageMapT},
		unhashed, StorageDoubleMap, StorageMap,
	},
	ReversibleStorageHasher, StorageHasher,
};
use sp_runtime::offchain::{
	storage_lock::{StorageLock, Time},
	Duration,
};
use sp_std::prelude::*;

/// Iterate over a prefix and decode raw_key and raw_value into `T`.
pub struct MapIterator<T> {
	prefix: Vec<u8>,
	previous_key: Vec<u8>,
	/// If true then value are removed while iterating
	drain: bool,
	/// Function that take `(raw_key_without_prefix, raw_value)` and decode `T`.
	/// `raw_key_without_prefix` is the raw storage key without the prefix
	/// iterated on.
	closure: fn(&[u8], &[u8]) -> Result<T, codec::Error>,
}

impl<T> Iterator for MapIterator<T> {
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		loop {
			let maybe_next = sp_io::storage::next_key(&self.previous_key).filter(|n| n.starts_with(&self.prefix));
			break match maybe_next {
				Some(next) => {
					self.previous_key = next;
					let raw_value = match unhashed::get_raw(&self.previous_key) {
						Some(raw_value) => raw_value,
						None => {
							frame_support::print("ERROR: next_key returned a key with no value in MapIterator");
							continue;
						}
					};
					if self.drain {
						unhashed::kill(&self.previous_key)
					}
					let raw_key_without_prefix = &self.previous_key[self.prefix.len()..];
					let item = match (self.closure)(raw_key_without_prefix, &raw_value[..]) {
						Ok(item) => item,
						Err(_e) => {
							frame_support::print("ERROR: (key, value) failed to decode in MapIterator");
							continue;
						}
					};

					Some(item)
				}
				None => None,
			};
		}
	}
}

/// A strongly-typed double map in storage whose secondary keys and values can
/// be iterated over.
pub trait RandomIterableStorageDoubleMap<K1: FullCodec, K2: FullCodec, V: FullCodec>:
	StorageDoubleMap<K1, K2, V>
{
	/// The type that iterates over all `(key2, value)`.
	type PrefixIterator: Iterator<Item = (K2, V)>;

	/// The type that iterates over all `(key1, key2, value)`.
	type Iterator: Iterator<Item = (K1, K2, V)>;

	/// Enumerate all elements in the map with first key `k1` in no particular
	/// order. If you add or remove values whose first key is `k1` to the map
	/// while doing this, you'll get undefined results.
	fn iter_prefix(k1: impl EncodeLike<K1>) -> Self::PrefixIterator;

	/// Remove all elements from the map with first key `k1` and iterate through
	/// them in no particular order. If you add elements with first key `k1` to
	/// the map while doing this, you'll get undefined results.
	fn drain_prefix(k1: impl EncodeLike<K1>) -> Self::PrefixIterator;

	/// Enumerate all elements in the map in no particular order. If you add or
	/// remove values to the map while doing this, you'll get undefined results.
	fn iter() -> Self::Iterator;

	/// Remove all elements from the map and iterate through them in no
	/// particular order. If you add elements to the map while doing this,
	/// you'll get undefined results.
	fn drain() -> Self::Iterator;

	/// Translate the values of all elements by a function `f`, in the map in no
	/// particular order. By returning `None` from `f` for an element, you'll
	/// remove it from the map.
	fn translate<O: Decode, F: Fn(O) -> Option<V>>(f: F);
}

impl<K1: FullCodec, K2: FullCodec, V: FullCodec, G: StorageDoubleMapT<K1, K2, V>>
	RandomIterableStorageDoubleMap<K1, K2, V> for G
where
	G::Hasher1: ReversibleStorageHasher,
	G::Hasher2: ReversibleStorageHasher,
{
	type PrefixIterator = MapIterator<(K2, V)>;
	type Iterator = MapIterator<(K1, K2, V)>;

	fn iter_prefix(k1: impl EncodeLike<K1>) -> Self::PrefixIterator {
		let prefix = G::storage_double_map_final_key1(k1);

		let random_seed = sp_io::offchain::random_seed();
		let random_key_hashed = G::Hasher2::hash(&random_seed);
		let mut start_key = Vec::with_capacity(prefix.len() + random_key_hashed.as_ref().len());
		start_key.extend_from_slice(&prefix[..]);
		start_key.extend_from_slice(random_key_hashed.as_ref());

		Self::PrefixIterator {
			prefix: prefix.clone(),
			previous_key: start_key,
			drain: false,
			closure: |raw_key_without_prefix, mut raw_value| {
				let mut key_material = G::Hasher2::reverse(raw_key_without_prefix);
				Ok((K2::decode(&mut key_material)?, V::decode(&mut raw_value)?))
			},
		}
	}

	fn drain_prefix(k1: impl EncodeLike<K1>) -> Self::PrefixIterator {
		let mut iterator = Self::iter_prefix(k1);
		iterator.drain = true;
		iterator
	}

	fn iter() -> Self::Iterator {
		let prefix = G::prefix_hash();

		let random_seed = sp_io::offchain::random_seed();
		let random_key_hashed = G::Hasher2::hash(&random_seed);
		let mut start_key = Vec::with_capacity(prefix.len() + random_key_hashed.as_ref().len());
		start_key.extend_from_slice(&prefix[..]);
		start_key.extend_from_slice(random_key_hashed.as_ref());

		Self::Iterator {
			prefix: prefix.clone(),
			previous_key: start_key,
			drain: false,
			closure: |raw_key_without_prefix, mut raw_value| {
				let mut k1_k2_material = G::Hasher1::reverse(raw_key_without_prefix);
				let k1 = K1::decode(&mut k1_k2_material)?;
				let mut k2_material = G::Hasher2::reverse(k1_k2_material);
				let k2 = K2::decode(&mut k2_material)?;
				Ok((k1, k2, V::decode(&mut raw_value)?))
			},
		}
	}

	fn drain() -> Self::Iterator {
		let mut iterator = Self::iter();
		iterator.drain = true;
		iterator
	}

	fn translate<O: Decode, F: Fn(O) -> Option<V>>(f: F) {
		let prefix = G::prefix_hash();

		let random_seed = sp_io::offchain::random_seed();
		let random_key_hashed = G::Hasher2::hash(&random_seed);
		let mut start_key = Vec::with_capacity(prefix.len() + random_key_hashed.as_ref().len());
		start_key.extend_from_slice(&prefix[..]);
		start_key.extend_from_slice(random_key_hashed.as_ref());

		let mut previous_key = start_key.clone();
		loop {
			match sp_io::storage::next_key(&previous_key).filter(|n| n.starts_with(&prefix)) {
				Some(next) => {
					previous_key = next;
					let maybe_value = unhashed::get::<O>(&previous_key);
					match maybe_value {
						Some(value) => match f(value) {
							Some(new) => unhashed::put::<V>(&previous_key, &new),
							None => unhashed::kill(&previous_key),
						},
						None => continue,
					}
				}
				None => return,
			}
		}
	}
}

pub fn iter_prefix_from_random_position_with_lock<
	K1: FullCodec,
	K2: FullCodec,
	V: FullCodec,
	G: StorageDoubleMapT<K1, K2, V>,
	F: Fn(K2, V) -> Result<(), OffchainErr>,
>(
	lock_duration: u64,
	lock_name: &[u8],
	max_iterations: u32,
	k1: K1,
	f: F,
) -> Result<(), OffchainErr>
where
	G::Hasher1: ReversibleStorageHasher,
	G::Hasher2: ReversibleStorageHasher,
{
	let lock_expiration = Duration::from_millis(lock_duration);
	let mut lock = StorageLock::<'_, Time>::with_deadline(lock_name, lock_expiration);

	// acquire offchain worker lock.
	let mut guard = lock.try_lock().map_err(|_| OffchainErr::OffchainLock)?;

	let mut count: u32 = 0;
	for (key2, value) in <G as RandomIterableStorageDoubleMap<K1, K2, V>>::iter_prefix(k1) {
		if count >= max_iterations {
			break;
		}
		count += 1;
		f(key2, value)?;
		guard.extend_lock().map_err(|_| OffchainErr::OffchainLock)?;
	}

	Ok(())
}

/// Utility to iterate through items in a storage map.
pub struct StorageMapIterator<K, V, Hasher> {
	prefix: Vec<u8>,
	previous_key: Vec<u8>,
	drain: bool,
	_phantom: ::sp_std::marker::PhantomData<(K, V, Hasher)>,
}

impl<K: Decode + Sized, V: Decode + Sized, Hasher: ReversibleStorageHasher> Iterator
	for StorageMapIterator<K, V, Hasher>
{
	type Item = (K, V);

	fn next(&mut self) -> Option<(K, V)> {
		loop {
			let maybe_next = sp_io::storage::next_key(&self.previous_key).filter(|n| n.starts_with(&self.prefix));
			break match maybe_next {
				Some(next) => {
					self.previous_key = next;
					match unhashed::get::<V>(&self.previous_key) {
						Some(value) => {
							if self.drain {
								unhashed::kill(&self.previous_key)
							}
							let mut key_material = Hasher::reverse(&self.previous_key[self.prefix.len()..]);
							match K::decode(&mut key_material) {
								Ok(key) => Some((key, value)),
								Err(_) => continue,
							}
						}
						None => continue,
					}
				}
				None => None,
			};
		}
	}
}

/// A strongly-typed map in storage whose keys and values can be iterated over.
pub trait RandomIterableStorageMap<K: FullEncode, V: FullCodec>: StorageMap<K, V> {
	/// The type that iterates over all `(key, value)`.
	type Iterator: Iterator<Item = (K, V)>;

	/// Enumerate all elements in the map in no particular order. If you alter
	/// the map while doing this, you'll get undefined results.
	fn iter() -> Self::Iterator;

	/// Remove all elements from the map and iterate through them in no
	/// particular order. If you add elements to the map while doing this,
	/// you'll get undefined results.
	fn drain() -> Self::Iterator;

	/// Translate the values of all elements by a function `f`, in the map in no
	/// particular order. By returning `None` from `f` for an element, you'll
	/// remove it from the map.
	fn translate<O: Decode, F: Fn(K, O) -> Option<V>>(f: F);
}

impl<K: FullCodec, V: FullCodec, G: StorageMapT<K, V>> RandomIterableStorageMap<K, V> for G
where
	G::Hasher: ReversibleStorageHasher,
{
	type Iterator = StorageMapIterator<K, V, G::Hasher>;

	/// Enumerate all elements in the map.
	fn iter() -> Self::Iterator {
		let prefix = G::prefix_hash();

		let random_seed = sp_io::offchain::random_seed();
		let random_key_hashed = G::Hasher::hash(&random_seed);
		let mut start_key = Vec::with_capacity(prefix.len() + random_key_hashed.as_ref().len());
		start_key.extend_from_slice(&prefix[..]);
		start_key.extend_from_slice(random_key_hashed.as_ref());

		Self::Iterator {
			prefix: prefix.clone(),
			previous_key: start_key,
			drain: false,
			_phantom: Default::default(),
		}
	}

	/// Enumerate all elements in the map.
	fn drain() -> Self::Iterator {
		let prefix = G::prefix_hash();

		let random_seed = sp_io::offchain::random_seed();
		let random_key_hashed = G::Hasher::hash(&random_seed);
		let mut start_key = Vec::with_capacity(prefix.len() + random_key_hashed.as_ref().len());
		start_key.extend_from_slice(&prefix[..]);
		start_key.extend_from_slice(random_key_hashed.as_ref());

		Self::Iterator {
			prefix: prefix.clone(),
			previous_key: start_key,
			drain: true,
			_phantom: Default::default(),
		}
	}

	fn translate<O: Decode, F: Fn(K, O) -> Option<V>>(f: F) {
		let prefix = G::prefix_hash();

		let random_seed = sp_io::offchain::random_seed();
		let random_key_hashed = G::Hasher::hash(&random_seed);
		let mut start_key = Vec::with_capacity(prefix.len() + random_key_hashed.as_ref().len());
		start_key.extend_from_slice(&prefix[..]);
		start_key.extend_from_slice(random_key_hashed.as_ref());

		let mut previous_key = start_key;
		loop {
			match sp_io::storage::next_key(&previous_key).filter(|n| n.starts_with(&prefix)) {
				Some(next) => {
					previous_key = next;
					let maybe_value = unhashed::get::<O>(&previous_key);
					match maybe_value {
						Some(value) => {
							let mut key_material = G::Hasher::reverse(&previous_key[prefix.len()..]);
							match K::decode(&mut key_material) {
								Ok(key) => match f(key, value) {
									Some(new) => unhashed::put::<V>(&previous_key, &new),
									None => unhashed::kill(&previous_key),
								},
								Err(_) => continue,
							}
						}
						None => continue,
					}
				}
				None => return,
			}
		}
	}
}

pub fn iter_from_random_position_with_lock<
	K: FullCodec,
	V: FullCodec,
	G: StorageMapT<K, V>,
	F: Fn(K, V) -> Result<(), OffchainErr>,
>(
	lock_duration: u64,
	lock_name: &[u8],
	max_iterations: u32,
	f: F,
) -> Result<(), OffchainErr>
where
	G::Hasher: ReversibleStorageHasher,
{
	let lock_expiration = Duration::from_millis(lock_duration);
	let mut lock = StorageLock::<'_, Time>::with_deadline(lock_name, lock_expiration);

	// acquire offchain worker lock.
	let mut guard = lock.try_lock().map_err(|_| OffchainErr::OffchainLock)?;

	let mut count: u32 = 0;
	for (key, value) in <G as RandomIterableStorageMap<K, V>>::iter() {
		if count >= max_iterations {
			break;
		}
		count += 1;
		f(key, value)?;
		guard.extend_lock().map_err(|_| OffchainErr::OffchainLock)?;
	}

	Ok(())
}
