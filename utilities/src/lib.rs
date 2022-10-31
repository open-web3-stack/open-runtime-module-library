#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::storage::{with_transaction, TransactionOutcome};
use sp_runtime::DispatchError;
use sp_std::result::Result;

#[deprecated(
	since = "0.4.1",
	note = "iterator module's functionality is now available in substrate's frame-support"
)]
pub mod iterator;
pub mod offchain_worker;
pub mod ordered_set;

#[allow(deprecated)]
pub use iterator::{IterableStorageDoubleMapExtended, IterableStorageMapExtended};

pub use offchain_worker::OffchainErr;
pub use ordered_set::OrderedSet;

/// Execute the supplied function in a new storage transaction.
///
/// All changes to storage performed by the supplied function are discarded if
/// the returned outcome is `Result::Err`.
///
/// Transactions can be nested to any depth. Commits happen to the parent
/// transaction.
pub fn with_transaction_result<R>(f: impl FnOnce() -> Result<R, DispatchError>) -> Result<R, DispatchError> {
	with_transaction(|| {
		let res = f();
		if res.is_ok() {
			TransactionOutcome::Commit(res)
		} else {
			TransactionOutcome::Rollback(res)
		}
	})
}

/// Simulate execution of the supplied function in a new storage transaction.
/// Changes to storage performed by the supplied function are always discarded.
pub fn simulate_execution<R>(f: impl FnOnce() -> Result<R, DispatchError>) -> Result<R, DispatchError> {
	with_transaction(|| {
		let res = f();
		TransactionOutcome::Rollback(res)
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::{assert_noop, assert_ok, decl_module, decl_storage};
	use sp_io::TestExternalities;
	use sp_runtime::{DispatchError, DispatchResult};
	use sp_std::result::Result;

	pub trait Config: frame_system::Config {}

	decl_module! {
		pub struct Module<T: Config> for enum Call where origin: T::RuntimeOrigin {}
	}

	decl_storage! {
		trait Store for Module<T: Config> as StorageTransactions {
			pub Value: u32;
			pub Map: map hasher(twox_64_concat) String => u32;
		}
	}

	#[test]
	fn storage_transaction_basic_commit() {
		TestExternalities::default().execute_with(|| {
			assert_eq!(Value::get(), 0);
			assert!(!Map::contains_key("val0"));

			assert_ok!(with_transaction_result(|| -> DispatchResult {
				Value::set(99);
				Map::insert("val0", 99);
				assert_eq!(Value::get(), 99);
				assert_eq!(Map::get("val0"), 99);
				Ok(())
			}));

			assert_eq!(Value::get(), 99);
			assert_eq!(Map::get("val0"), 99);
		});
	}

	#[test]
	fn storage_transaction_basic_rollback() {
		TestExternalities::default().execute_with(|| {
			assert_eq!(Value::get(), 0);
			assert_eq!(Map::get("val0"), 0);

			assert_noop!(
				with_transaction_result(|| -> DispatchResult {
					Value::set(99);
					Map::insert("val0", 99);
					assert_eq!(Value::get(), 99);
					assert_eq!(Map::get("val0"), 99);
					Err("test".into())
				}),
				DispatchError::Other("test")
			);

			assert_eq!(Value::get(), 0);
			assert_eq!(Map::get("val0"), 0);
		});
	}

	#[test]
	fn simulate_execution_works() {
		TestExternalities::default().execute_with(|| {
			assert_eq!(Value::get(), 0);
			assert_eq!(Map::get("val0"), 0);

			// Roll back on `Err`.
			assert_noop!(
				simulate_execution(|| -> DispatchResult {
					Value::set(99);
					Map::insert("val0", 99);
					Err(DispatchError::Other("test"))
				}),
				DispatchError::Other("test")
			);
			assert_eq!(Value::get(), 0);
			assert_eq!(Map::get("val0"), 0);

			// Roll back on `Ok`, but returns `Ok` result.
			assert_ok!(
				simulate_execution(|| -> Result<u32, DispatchError> {
					Value::set(99);
					Map::insert("val0", 99);
					Ok(99)
				}),
				99
			);
			assert_eq!(Value::get(), 0);
			assert_eq!(Map::get("val0"), 0);
		});
	}
}
