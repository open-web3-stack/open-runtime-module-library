/// Run benches in WASM environment.
///
/// Update Cargo.toml by adding:
/// ```toml
/// ..
/// [package]
/// name = "your-module"
/// ..
/// [[bench]]
/// name = 'your-module-benches'
/// harness = false
/// path = 'src/benches.rs'
/// required-features = ['bench']
///
/// [features]
/// bench = ['orml-weight-meter/bench']
/// ..
/// ```
///
/// Define benches
///
/// Create a file `src/benches.rs`:
/// ```.ignore
/// #![allow(dead_code)]
///
/// use orml_bencher::{Bencher, bench};
/// use your_module::mock::{Block, YourModule};
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
/// bench!(Block, foo, bar);
/// ```
/// Update `src/lib.rs`:
/// ```.ignore
/// #[cfg(any(feature = "bench", test))]
/// pub mod mock; /* mock runtime needs to be compiled into wasm */
/// #[cfg(feature = "bench")]
/// pub mod benches;
///
/// extern crate self as your_module;
/// ```
///
/// Run benchmarking: `cargo bench --features=bench`
#[macro_export]
macro_rules! bench {
    (
        $block:tt,
        $($method:path),+
    ) => {
        $crate::sp_core::wasm_export_functions! {
            fn run_benches() -> $crate::sp_std::vec::Vec<$crate::BenchResult> {
                let mut bencher = $crate::Bencher::default();
                $(
                    bencher.reset();
                    $method(&mut bencher);
                )+
                bencher.results
            }
        }

        #[cfg(all(feature = "std", feature = "bench"))]
        pub fn main() -> std::io::Result<()> {
            let wasm = $crate::build_wasm::build()?;
            let output = $crate::bench_runner::run::<$block>(wasm);
			$crate::handler::handle(output);
            Ok(())
        }
    }
}
