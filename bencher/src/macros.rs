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
/// bench = [
///    'orml-bencher/bench'
///    'orml-weight-meter/bench'
/// ]
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
///     b.prepare(|| {
///         // optional. prepare block, runs before bench
///     })
///     .bench(|| {
///         // foo must have macro `[orml_weight_meter::weight(..)]`
///         YourModule::foo();
///     })
///     .verify(|| {
///         // optional. verify block, runs before bench
///     });
/// }
///
/// fn bar(b: &mut Bencher) {
///     // optional. method name is used by default i.e: `bar`
///     b.name("bench_name")
///     .bench(|| {
///         // bar must have macro `[orml_weight_meter::weight(..)]`
///         YourModule::bar();
///     });
/// }
///
/// bench!(Block, foo, bar); // Tests are generated automatically
/// ```
/// Update `src/lib.rs`:
/// ```.ignore
/// #[cfg(any(feature = "bench", test))]
/// pub mod mock; /* mock runtime needs to be compiled into wasm */
/// #[cfg(any(feature = "bench", test))]
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
        #[cfg(feature = "bench")]
        $crate::sp_core::wasm_export_functions! {
            fn run_benches() -> $crate::sp_std::vec::Vec<$crate::BenchResult> {
                let mut bencher = $crate::Bencher::default();
                $(
                    bencher.reset();
                    $method(&mut bencher);
                    if bencher.name.len() == 0 {
                        // use method name as default bench name
                        bencher.name(stringify!($method));
                    }
                    bencher.run();
                )+
                bencher.results
            }
        }

        #[cfg(all(feature = "bench", not(feature = "std")))]
        #[panic_handler]
        #[no_mangle]
        fn panic_handler(info: &::core::panic::PanicInfo) -> ! {
            unsafe {
                let message = $crate::sp_std::alloc::format!("{}", info);
                $crate::bencher::panic(message.as_bytes().to_vec());
                core::arch::wasm32::unreachable();
            }
        }

        #[cfg(all(feature = "std", feature = "bench"))]
        pub fn main() -> std::io::Result<()> {
            let wasm = $crate::build_wasm::build()?;
            match $crate::bench_runner::run::<$block>(wasm) {
                Ok(output) => { $crate::handler::handle(output); }
                Err(e) => { eprintln!("{:?}", e); }
            };
            Ok(())
        }

        // Tests
        $(
            $crate::paste::item! {
                #[test]
                fn [<test_ $method>] () {
                    $crate::sp_io::TestExternalities::new_empty().execute_with(|| {
                        let mut bencher = $crate::Bencher::default();
                        $method(&mut bencher);
                        bencher.run();
                    });
                }
            }
        )+

    }
}
