/// Define benches
///
/// Create a file `src/benches.rs`:
/// ```.ignore
/// #![cfg_attr(not(feature = "std"), no_std)]
/// #![allow(dead_code)]
///
/// #[cfg(feature = "std")]
/// orml_bencher::bencher_use!(
///     crate::mock::wasm_binary_unwrap,
///     crate::mock::Block,
///     crate::mock::Hasher,
///     crate::mock::BlockNumber
/// );
///
/// use crate::mock::YourModule;
///
/// fn foo(b: &mut Bencher) {
///     b.bench("foo", || {
///         YourModule::foo();
///     });
/// }
///
/// fn bar(b: &mut Bencher) {
///     b.bench("bar", || {
///         YourModule::bar();
///     });
/// }
///
/// orml_bencher::bench!(foo, bar);
/// ```
/// Update `src/lib.rs`:
/// ```.ignore
/// #[cfg(any(feature = "bench", test))]
/// pub mod mock; /* mock runtime needs to be compiled into wasm */
/// #[cfg(feature = "bench")]
/// pub mod benches;
/// ```
#[macro_export]
macro_rules! bench {
    (
        $($method:path),+
    ) => {
        use $crate::codec::{Encode, Decode};
        use $crate::sp_std::{cmp::max, prelude::Vec};
        use $crate::frame_benchmarking::{benchmarking, BenchmarkResults};

        #[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
        pub struct BenchResult {
            pub method: Vec<u8>,
            pub elapses: Vec<u128>,
            pub reads: u32,
            pub repeat_reads: u32,
            pub writes: u32,
            pub repeat_writes: u32,
        }

        #[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
        pub struct Bencher {
            pub results: Vec<BenchResult>,
        }

        impl Bencher {
            pub fn bench(&mut self, name: &str, block: fn() -> ()) {
                // Warm up the DB
                benchmarking::commit_db();
                benchmarking::wipe_db();

                let mut result = BenchResult {
                    method: name.as_bytes().to_vec(),
                    ..Default::default()
                };

                for _ in 0..50 {
                    benchmarking::commit_db();
                    benchmarking::reset_read_write_count();

                    let start_time = benchmarking::current_time();
                    block();
                    let end_time = benchmarking::current_time();
                    let elasped = end_time - start_time;
                    result.elapses.push(elasped);

                    benchmarking::commit_db();
                    let (reads, repeat_reads, writes, repeat_writes) = benchmarking::read_write_count();

                    result.reads = max(result.reads, reads);
                    result.repeat_reads = max(result.repeat_reads, repeat_reads);
                    result.writes = max(result.writes, writes);
                    result.repeat_writes = max(result.repeat_writes, repeat_writes);

                    benchmarking::wipe_db();
                }
                self.results.push(result);
            }
        }

        $crate::sp_core::wasm_export_functions! {
            fn run_benches() -> Bencher {
                let mut bencher = Bencher::default();
                $(
                    $method(&mut bencher);
                )+
                bencher
            }
        }
    }
}
