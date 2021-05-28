/// Run benches in WASM environment.
///
/// Configure your module to build the mock runtime into wasm code.
/// Create a `build.rs` like you do with your runtime.
/// ```.ignore
/// use substrate_wasm_builder::WasmBuilder;
/// fn main() {
///     WasmBuilder::new()
///         .with_current_project()
///         .export_heap_base()
///         .import_memory()
///         .build()
/// }
/// ```
///
/// Update mock runtime to be build into wasm code.
/// ```.ignore
/// #![cfg_attr(not(feature = "std"), no_std)]
///
/// #[cfg(feature = "std")]
/// include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));
///
/// #[cfg(feature = "std")]
/// pub fn wasm_binary_unwrap() -> &'static [u8] { WASM_BINARY.unwrap() }
/// ..
/// ```
///
/// Create a file `bench_runner.rs` with following code:
///  ```.ignore
/// orml_bencher::run_benches!(my_module::benches);
/// ```
/// 
/// Update Cargo.toml by adding:
/// ```toml
/// ..
/// [package]
/// name = "my-module"
/// ..
/// build = 'build.rs'
///
/// [build-dependencies]
/// substrate-wasm-builder = '4.0.0'
///
/// [[bench]]
/// name = 'my-module-benches'
/// harness = false
/// path = 'bench_runner.rs'
/// required-features = ['bench']
///
/// [features]
/// bench = ['orml-weight-meter/bench']
/// ..
/// ```
/// 
/// Run bench with features bench: `cargo bench --features=bench`
#[cfg(feature = "std")]
#[macro_export]
macro_rules! run_benches {
	($benches:path) => {
		use $benches::{wasm_binary_unwrap, Block};
		pub fn main() {
			let output = $crate::bench_runner::run::<Block>(wasm_binary_unwrap().to_vec());
			$crate::handler::handle(output);
		}
	};
}

/// Define benches
///
/// Create a file `src/benches.rs`:
/// ```.ignore
/// #![cfg_attr(not(feature = "std"), no_std)]
/// #![allow(dead_code)]
///
/// #[cfg(feature = "std")] // Re-export for bench_runner
/// pub use crate::mock::{Block, wasm_binary_unwrap};
///
/// use crate::mock::YourModule;
///
/// fn foo(b: &mut Bencher) {
///     b.set_prepare(|| {
///         // optional. prepare block, run before bench
///     });
///
///     b.set_verify(|| {
///         // optional. verify block, run before bench
///     });
///
///     // Add macro `[orml_weight_meter::weight(..)]` for method `foo` before running bench
///     b.bench("foo", || {
///         YourModule::foo();
///     });
/// }
///
/// fn bar(b: &mut Bencher) {
///     // Add macro `[orml_weight_meter::weight(..)]` for method `bar` before running bench
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
        use $crate::BenchResult;
        use $crate::sp_std::{cmp::max, prelude::{Box, Vec}};
        use $crate::frame_support::log;
        use $crate::frame_benchmarking::{benchmarking, BenchmarkResults};

        struct Bencher {
            pub results: Vec<BenchResult>,
            pub prepare: Box<dyn Fn() -> ()>,
            pub verify: Box<dyn Fn() -> ()>,
        }

        impl Default for Bencher {
            fn default() -> Self {
                Bencher {
                    results: Vec::new(),
                    prepare: Box::new(|| {}),
                    verify: Box::new(|| {}),
                }
            }
        }

        impl Bencher {
            /// Reset prepare and verify block
            pub fn reset(&mut self) {
                self.prepare = Box::new(|| {});
                self.verify = Box::new(|| {});
            }

            /// Set prepare block
            pub fn set_prepare(&mut self, prepare: impl Fn() -> () + 'static) -> &mut Self {
                self.prepare = Box::new(prepare);
                self
            }

            /// Set verify block
            pub fn set_verify(&mut self, verify: impl Fn() -> () + 'static) -> &mut Self {
                self.verify = Box::new(verify);
                self
            }

            /// Run benchmark for block
            pub fn bench<F: Fn() -> ()>(&mut self, name: &str, block: F) {
                // Warm up the DB
                benchmarking::commit_db();
                benchmarking::wipe_db();

                let mut result = BenchResult {
                    method: name.as_bytes().to_vec(),
                    ..Default::default()
                };

                for _ in 0..50 {
                    // Execute prepare block
                    (self.prepare)();

                    benchmarking::commit_db();
                    benchmarking::reset_read_write_count();

                    let start_time = benchmarking::current_time();
                    // Execute bench block
                    block();
                    let end_time = benchmarking::current_time();
                    benchmarking::commit_db();

                    let (elapsed, reads, repeat_reads, writes, repeat_writes) = $crate::bencher::finalized_results(end_time - start_time);

                    // Execute verify block
                    (self.verify)();

                    // Reset the DB
                    benchmarking::wipe_db();

                    result.elapses.push(elapsed);

                    result.reads = max(result.reads, reads);
                    result.repeat_reads = max(result.repeat_reads, repeat_reads);
                    result.writes = max(result.writes, writes);
                    result.repeat_writes = max(result.repeat_writes, repeat_writes);
                }
                self.results.push(result);
            }
        }

        $crate::sp_core::wasm_export_functions! {
            fn run_benches() -> Vec<BenchResult> {
                let mut bencher = Bencher::default();
                $(
                    bencher.reset();
                    $method(&mut bencher);
                )+
                bencher.results
            }
        }
    }
}
