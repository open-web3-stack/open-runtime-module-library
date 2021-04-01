#[macro_export]
macro_rules! run_benchmarks {
	($wasm:path, $benches:path) => {
		mod benchmark_runner {
			use sc_executor::{sp_wasm_interface::HostFunctions, WasmExecutionMethod, WasmExecutor};
			use $benches::{benchmark_api, Bencher};
			use $crate::codec::Decode;
			use $crate::linregress::{FormulaRegressionBuilder, RegressionDataBuilder};
			use $crate::sp_core::traits::{CallInWasm, MissingHostFunctions};
			use $crate::sp_io::{SubstrateHostFunctions, TestExternalities};

			pub fn run() {
				let mut ext = TestExternalities::default();
				let mut ext = ext.ext();

				let mut host_functions = benchmark_api::HostFunctions::host_functions();
				host_functions.append(&mut SubstrateHostFunctions::host_functions());

				let executor = WasmExecutor::new(WasmExecutionMethod::Compiled, Some(1024), host_functions, 1, None);

				let output = executor
					.call_in_wasm(
						&$wasm()[..],
						None,
						"run_benchmarks",
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
			benchmark_runner::run();
		}
	};
}
