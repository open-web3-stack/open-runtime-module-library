//! Tests for the module.

#![cfg(test)]

use super::*;
use frame_benchmarking::account;
use frame_support::{assert_err, assert_ok, construct_runtime, ensure};
use frame_system::RawOrigin;
use sp_runtime::{
	testing::{Header, H256},
	traits::{BlakeTwo256, IdentityLookup},
};
use sp_std::prelude::*;

mod test {
	use frame_support::{decl_module, decl_storage, dispatch::DispatchResult};
	use frame_system::{ensure_none, ensure_signed};
	use sp_std::prelude::*;

	pub trait Config: frame_system::Config {
		type Event;
		type BlockNumber;
	}

	decl_storage! {
		trait Store for Module<T: Config> as Test {
			pub Value get(fn value) config(): Option<u32>;
		}
	}

	decl_module! {
		pub struct Module<T: Config> for enum Call where origin: T::Origin {
			#[weight = 0]
			fn set_value(origin, n: u32) -> DispatchResult {
				let _sender = ensure_signed(origin)?;
				Value::put(n);
				Ok(())
			}

			#[weight = 0]
			fn dummy(origin, _n: u32) -> DispatchResult {
				let _sender = ensure_none(origin)?;
				Ok(())
			}
		}
	}
}

pub trait Config: frame_system::Config {
	type Event;
	type BlockNumber;
}

type AccountId = u128;

impl frame_system::Config for Test {
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Call = Call;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = ();
	type DbWeight = ();
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
}

impl tests::test::Config for Test {
	type Event = Event;
	type BlockNumber = u32;
}

impl Config for Test {
	type Event = Event;
	type BlockNumber = u32;
}

pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<u32, Call, u32, ()>;

construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		Pallet: test::{Pallet, Call, Storage, Config},
	}
);

// This function basically just builds a genesis storage key/value store
// according to our desired mockup.
fn new_test_ext() -> sp_io::TestExternalities {
	frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap()
		.into()
}

runtime_benchmarks! {
	{ Test, test }

	set_value {
		let b in 1 .. 1000;
		let caller = account::<AccountId>("caller", 0, 0);
	}: _ (RawOrigin::Signed(caller), b.into())
	verify {
		assert_eq!(Pallet::value(), Some(b));
	}

	other_name {
		let b in 1 .. 1000;
	}: dummy (RawOrigin::None, b.into())

	sort_vector {
		let x in 1 .. 10000;
		let mut m = Vec::<u32>::new();
		for i in (0..x).rev() {
			m.push(i);
		}
	}: {
		m.sort();
	} verify {
		ensure!(m[0] == 0, "You forgot to sort!")
	}

	bad_origin {
		let b in 1 .. 1000;
		let caller = account::<AccountId>("caller", 0, 0);
	}: dummy (RawOrigin::Signed(caller), b.into())

	bad_verify {
		let x in 1 .. 10000;
		let mut m = Vec::<u32>::new();
		for i in (0..x).rev() {
			m.push(i);
		}
	}: { }
	verify {
		ensure!(m[0] == 0, "You forgot to sort!")
	}
}

#[test]
fn benchmarks_macro_works() {
	// Check benchmark creation for `set_value`.
	let selected_benchmark = SelectedBenchmark::set_value;

	let components = <SelectedBenchmark as BenchmarkingSetup<Test>>::components(&selected_benchmark);
	assert_eq!(components, vec![(BenchmarkParameter::b, 1, 1000)]);

	let closure = <SelectedBenchmark as BenchmarkingSetup<Test>>::instance(
		&selected_benchmark,
		&[(BenchmarkParameter::b, 1)],
		true,
	)
	.expect("failed to create closure");

	new_test_ext().execute_with(|| {
		assert_eq!(closure(), Ok(()));
	});
}

#[test]
fn benchmarks_macro_rename_works() {
	// Check benchmark creation for `other_dummy`.
	let selected_benchmark = SelectedBenchmark::other_name;
	let components = <SelectedBenchmark as BenchmarkingSetup<Test>>::components(&selected_benchmark);
	assert_eq!(components, vec![(BenchmarkParameter::b, 1, 1000)]);

	let closure = <SelectedBenchmark as BenchmarkingSetup<Test>>::instance(
		&selected_benchmark,
		&[(BenchmarkParameter::b, 1)],
		true,
	)
	.expect("failed to create closure");

	new_test_ext().execute_with(|| {
		assert_ok!(closure());
	});
}

#[test]
fn benchmarks_macro_works_for_non_dispatchable() {
	let selected_benchmark = SelectedBenchmark::sort_vector;

	let components = <SelectedBenchmark as BenchmarkingSetup<Test>>::components(&selected_benchmark);
	assert_eq!(components, vec![(BenchmarkParameter::x, 1, 10000)]);

	let closure = <SelectedBenchmark as BenchmarkingSetup<Test>>::instance(
		&selected_benchmark,
		&[(BenchmarkParameter::x, 1)],
		true,
	)
	.expect("failed to create closure");

	assert_eq!(closure(), Ok(()));
}

#[test]
fn benchmarks_macro_verify_works() {
	// Check postcondition for benchmark `set_value` is valid.
	let selected_benchmark = SelectedBenchmark::set_value;

	let closure = <SelectedBenchmark as BenchmarkingSetup<Test>>::instance(
		&selected_benchmark,
		&[(BenchmarkParameter::b, 1)],
		true,
	)
	.expect("failed to create closure");

	new_test_ext().execute_with(|| {
		assert_ok!(closure());
	});

	// Check postcondition for benchmark `bad_verify` is invalid.
	let selected = SelectedBenchmark::bad_verify;

	let closure =
		<SelectedBenchmark as BenchmarkingSetup<Test>>::instance(&selected, &[(BenchmarkParameter::x, 10000)], true)
			.expect("failed to create closure");

	new_test_ext().execute_with(|| {
		assert_err!(closure(), "You forgot to sort!");
	});
}

#[test]
fn benchmarks_generate_unit_tests() {
	new_test_ext().execute_with(|| {
		assert_ok!(test_benchmark_set_value());
		assert_ok!(test_benchmark_other_name());
		assert_ok!(test_benchmark_sort_vector());
		assert_err!(test_benchmark_bad_origin(), "Bad origin");
		assert_err!(test_benchmark_bad_verify(), "You forgot to sort!");
	});
}
