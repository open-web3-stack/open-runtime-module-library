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
/// orml_bencher::run_benches!(
///    pallet_template::mock::wasm_binary_unwrap, /* path to method returning
/// wasm mock runtime */    pallet_template::benches /* path to benches */
/// );
/// ```
/// 
/// Update Cargo.toml by adding:
/// ```toml
/// ..
/// edition = '2018'
/// version = '3.0.0'
/// build = 'build.rs'
///
/// [build-dependencies]
/// substrate-wasm-builder = '4.0.0'
///
/// [[bench]]
/// name = 'benches'
/// harness = false
/// path = 'bench_runner.rs'
/// required-features = ['bench']
///
/// [features]
/// bench = []
/// ..
/// ```
/// 
/// Run bench with features bench: `cargo bench --features=bench`
#[macro_export]
macro_rules! run_benches {
	($wasm:path, $benches:path) => {
		mod bench_runner {
			use $benches::{bench_api, Bencher};
			use $crate::codec::Decode;
			use $crate::linregress::{FormulaRegressionBuilder, RegressionDataBuilder};
			use $crate::sc_executor::{sp_wasm_interface::HostFunctions, WasmExecutionMethod, WasmExecutor};
			use $crate::sp_core::traits::{CallInWasm, MissingHostFunctions};
			use $crate::sp_io::{SubstrateHostFunctions, TestExternalities};

			pub fn run() {
				let mut ext = TestExternalities::default();
				let mut ext = ext.ext();

				let mut host_functions = bench_api::HostFunctions::host_functions();
				host_functions.append(&mut SubstrateHostFunctions::host_functions());

				let executor = WasmExecutor::new(WasmExecutionMethod::Compiled, Some(1024), host_functions, 1, None);

				let output = executor
					.call_in_wasm(
						&$wasm()[..],
						None,
						"run_benches",
						&[],
						&mut ext,
						MissingHostFunctions::Allow,
					)
					.unwrap();

				let Bencher { results } = <Bencher as Decode>::decode(&mut &output[..]).unwrap();

				for result in results {
					let method = String::from_utf8_lossy(&result.method);

					let y: Vec<f64> = result.elapses.into_iter().map(|x| x as f64).collect();
					eprintln!("Elapses: {:#?}", y);
					let x: Vec<f64> = (0..50).into_iter().map(|x| x as f64).collect();
					let data = vec![("Y", y), ("X", x)];
					let data = RegressionDataBuilder::new().build_from(data).unwrap();
					let formula = "Y ~ X";

					let formula = FormulaRegressionBuilder::new().data(&data).formula(formula).fit();

					match formula {
						Ok(model) => eprintln!(
							"Method: {:?} ~ {}ns",
							method, model.parameters.intercept_value as u64
						),
						Err(e) => println!("Method: {:?} Error: {:?}", method, e),
					};
				}
			}
		}

		pub fn main() {
			bench_runner::run();
		}
	};
}
