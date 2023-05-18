/// Run benches in WASM environment.
///
/// Update Cargo.toml by adding:
/// ```toml
/// ..
/// [package]
/// name = "your-module"
/// ..
/// [[bench]]
/// name = 'module_benches'
/// harness = false
/// required-features = ['wasm-bench']
///
/// [features]
/// wasm-bench = [
///    'orml-bencher/wasm-bench'
///    'orml-weight-meter/wasm-bench'
/// ]
/// ..
/// ```
///
/// Create a file `benches/module_benches.rs` must be the same as bench name.
/// ```.ignore
/// use your_module::mock::{Runtime, AllPalletsWithSystem};
/// orml_bencher::main!(Runtime, AllPalletsWithSystem);
/// ```
///
/// Define benches
///
/// Create a file `src/benches.rs`
/// ```ignore
/// #!#[cfg(feature = "wasm-bench")]
///
/// use orml_bencher::{Bencher, benches};
/// use crate::mock::*;
///
/// fn foo(b: &mut Bencher) {
///     // Run anything before code here
///     let ret = b.bench(|| {
///         // foo must have macro `[orml_weight_meter::weight(..)]` to measure correct redundant info
///         YourModule::foo()
///     });
///     // Run anything after code here
/// }
///
/// fn bar(b: &mut Bencher) {
///     // optional. method name is used by default i.e: `bar`
///     b.name("bench_name")
///     .bench(|| {
///         // bar must have macro `[orml_weight_meter::weight(..)]` to measure correct redundant info
///         YourModule::bar();
///     });
/// }
///
/// benches!(foo, bar); // Tests are generated automatically
/// ```
/// Update `src/lib.rs`
/// ```ignore
/// #[cfg(any(feature = "wasm-bench", test))]
/// pub mod mock; /* mock runtime needs to be compiled into wasm */
/// pub mod benches;
/// ```
///
/// Run benchmarking: `cargo bench --features=wasm-bench`
/// Run tests & benchmarking: `cargo bench --features=wasm-bench -- --test`
/// Run only tests: `cargo test --features=wasm-bench`
#[macro_export]
macro_rules! benches {
    ($($method:path),+) => {
        #[cfg(feature = "wasm-bench")]
        $crate::paste::item! {
            use $crate::sp_std::vec::Vec;
            $crate::sp_core::wasm_export_functions! {
                // list of bench methods
                fn available_bench_methods() -> Vec<&str> {
                    let mut methods = Vec::<&str>::new();
                    $(
                        methods.push(stringify!($method));
                    )+
                    methods.sort();
                    methods
                }

                // wrapped bench methods to run
                $(
                    fn [<bench_ $method>] () -> $crate::Bencher {
                        let name = stringify!($method);
                        let mut bencher = $crate::Bencher::with_name(name);

                        for _ in 0..1_000 {
                            bencher.before_run();
                            $method(&mut bencher);
                        }

                        bencher
                    }
                )+
            }
        }


        #[cfg(all(feature = "wasm-bench", not(feature = "std")))]
        #[panic_handler]
        #[no_mangle]
        fn panic_handler(info: &::core::panic::PanicInfo) -> ! {
            let message = $crate::sp_std::alloc::format!("{}", info);
            $crate::bench::print_error(message.as_bytes().to_vec());
            unsafe {core::arch::wasm32::unreachable(); }
        }

        // Tests
        #[cfg(test)]
        mod tests {
            $(
                $crate::paste::item! {
                    #[test]
                    fn [<bench_ $method>] () {
                        $crate::sp_io::TestExternalities::new_empty().execute_with(|| {
                            let mut bencher = $crate::Bencher::default();
                            super::$method(&mut bencher);
                        });
                    }
                }
            )+
        }

    }
}

#[macro_export]
macro_rules! main {
	(
        $runtime:ident
        $(,$all_pallets_with_system:ident)?
    ) => {
		#[cfg(all(feature = "std", feature = "wasm-bench"))]
		pub fn main() -> std::io::Result<()> {
			use $crate::frame_support::{sp_runtime::traits::GetRuntimeBlockType, traits::StorageInfoTrait};
			let wasm = $crate::build_wasm::build()?;
            type Block = <$runtime as GetRuntimeBlockType>::RuntimeBlock;

            let methods = $crate::bench_runner::run::<Block>(&wasm[..], "available_bench_methods", &[]).unwrap();
            let bench_methods = <Vec<String> as codec::Decode>::decode(&mut &methods[..]).unwrap();
            println!("\nRunning {} benches\n", bench_methods.len());

            let mut results: Vec<$crate::handler::BenchData> = vec![];
            let mut failed: Vec<String> = vec![];

            for method in bench_methods {
                $crate::handler::print_start(&method);
                match $crate::bench_runner::run::<Block>(&wasm[..], &format!("bench_{method}"), &[])
                {
                    Ok(output) => {
                        let data = $crate::handler::parse(output);
                        $crate::handler::print_summary(&data);
                        results.push(data);
                    }
                    Err(err) => {
                        failed.push(method);
                    }
                };
            }

            if failed.is_empty() {
                println!("\n✅ Complete: {}", $crate::colorize::green_bold(&format!("{} passed", results.len())));
            } else {
                println!("\n❌ Finished with errors: {}, {}", $crate::colorize::green_bold(&format!("{} passed", results.len())), $crate::colorize::red_bold(&format!("{} failed", failed.len())));
                std::process::exit(1);
            }

            let mut storage_info: Vec<$crate::frame_support::traits::StorageInfo> = vec![];
            $(storage_info = $all_pallets_with_system::storage_info();)?
            if std::env::args().find(|x| x.eq("json")).is_some() {
                assert!(!storage_info.is_empty(), "Cannot find storage info, please include `AllPalletsWithSystem` generated by `frame_support::construct_runtime`");
                $crate::handler::save_output_json(results, storage_info);
            }

			Ok(())
		}
	};
}
