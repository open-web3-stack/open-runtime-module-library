#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::storage::{with_transaction, TransactionOutcome};
use sp_runtime::DispatchError;
use sp_std::result::Result;

pub mod offchain_worker;
pub mod ordered_set;

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
	use frame_support::{assert_noop, assert_ok, construct_runtime, derive_impl, pallet_prelude::*};
	use sp_io::TestExternalities;
	use sp_runtime::traits::IdentityLookup;
	use sp_runtime::{DispatchError, DispatchResult};
	use sp_std::result::Result;

	#[allow(dead_code)]
	#[frame_support::pallet]
	pub mod module {
		use super::*;

		#[pallet::config]
		pub trait Config: frame_system::Config {}

		#[pallet::pallet]
		pub struct Pallet<T>(_);

		#[pallet::storage]
		pub type Value<T: Config> = StorageValue<_, u32, ValueQuery>;

		#[pallet::storage]
		pub type Map<T: Config> = StorageMap<_, Twox64Concat, [u8; 4], u32, ValueQuery>;
	}

	use module::*;

	#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
	impl frame_system::Config for Runtime {
		type AccountId = u128;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Block = Block;
	}

	impl module::Config for Runtime {}

	type Block = frame_system::mocking::MockBlock<Runtime>;

	construct_runtime!(
		pub enum Runtime {
			System: frame_system,
			TestModule: module,
		}
	);

	#[test]
	fn storage_transaction_basic_commit() {
		TestExternalities::default().execute_with(|| {
			assert_eq!(Value::<Runtime>::get(), 0);
			assert!(!Map::<Runtime>::contains_key(b"val0"));

			assert_ok!(with_transaction_result(|| -> DispatchResult {
				Value::<Runtime>::set(99);
				Map::<Runtime>::insert(b"val0", 99);
				assert_eq!(Value::<Runtime>::get(), 99);
				assert_eq!(Map::<Runtime>::get(b"val0"), 99);
				Ok(())
			}));

			assert_eq!(Value::<Runtime>::get(), 99);
			assert_eq!(Map::<Runtime>::get(b"val0"), 99);
		});
	}

	#[test]
	fn storage_transaction_basic_rollback() {
		TestExternalities::default().execute_with(|| {
			assert_eq!(Value::<Runtime>::get(), 0);
			assert_eq!(Map::<Runtime>::get(b"val0"), 0);

			assert_noop!(
				with_transaction_result(|| -> DispatchResult {
					Value::<Runtime>::set(99);
					Map::<Runtime>::insert(b"val0", 99);
					assert_eq!(Value::<Runtime>::get(), 99);
					assert_eq!(Map::<Runtime>::get(b"val0"), 99);
					Err("test".into())
				}),
				DispatchError::Other("test")
			);

			assert_eq!(Value::<Runtime>::get(), 0);
			assert_eq!(Map::<Runtime>::get(b"val0"), 0);
		});
	}

	#[test]
	fn simulate_execution_works() {
		TestExternalities::default().execute_with(|| {
			assert_eq!(Value::<Runtime>::get(), 0);
			assert_eq!(Map::<Runtime>::get(b"val0"), 0);

			// Roll back on `Err`.
			assert_noop!(
				simulate_execution(|| -> DispatchResult {
					Value::<Runtime>::set(99);
					Map::<Runtime>::insert(b"val0", 99);
					Err(DispatchError::Other("test"))
				}),
				DispatchError::Other("test")
			);
			assert_eq!(Value::<Runtime>::get(), 0);
			assert_eq!(Map::<Runtime>::get(b"val0"), 0);

			// Roll back on `Ok`, but returns `Ok` result.
			assert_ok!(
				simulate_execution(|| -> Result<u32, DispatchError> {
					Value::<Runtime>::set(99);
					Map::<Runtime>::insert(b"val0", 99);
					Ok(99)
				}),
				99
			);
			assert_eq!(Value::<Runtime>::get(), 0);
			assert_eq!(Map::<Runtime>::get(b"val0"), 0);
		});
	}
}
