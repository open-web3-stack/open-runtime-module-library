#![cfg_attr(not(feature = "std"), no_std)]

pub extern crate codec;
#[cfg(feature = "std")]
pub use sp_core::traits::{CallInWasm, MissingHostFunctions};
#[cfg(feature = "std")]
pub extern crate linregress;
pub extern crate sp_core;
#[cfg(feature = "std")]
pub extern crate sp_io;
pub extern crate sp_runtime_interface;
pub extern crate sp_std;

#[cfg(feature = "std")]
#[macro_export]
macro_rules! run_benchmarks {
	($wasm:path, $benches:path) => {
		mod benchmark_runner {
			use sc_executor::{sp_wasm_interface::HostFunctions, WasmExecutionMethod, WasmExecutor};
			use $benches::{benchmark_api, Bencher};
			use $crate::codec::Decode;
			use $crate::linregress::{FormulaRegressionBuilder, RegressionDataBuilder};
			use $crate::sp_io::{SubstrateHostFunctions, TestExternalities};
			use $crate::{CallInWasm, MissingHostFunctions};

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

#[macro_export]
macro_rules! bench {
    (
        $($method:path),+
    ) => {
        use $crate::codec::{Encode, Decode};
        use $crate::sp_std::prelude::*;
        #[$crate::sp_runtime_interface::runtime_interface]
        pub trait BenchmarkApi {
            fn current_time() -> u128 {
                ::std::time::SystemTime::now().duration_since(::std::time::UNIX_EPOCH)
                    .expect("Unix time doesn't go backwards; qed")
                    .as_nanos()
            }
        }

        #[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
        pub struct BenchResult {
            pub method: Vec<u8>,
            pub elapses: Vec<u128>,
        }

        #[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
        pub struct Bencher {
            pub results: Vec<BenchResult>,
        }

        impl Bencher {
            pub fn bench(&mut self, name: &str, block: fn() -> ()) {
                let mut elapses: Vec<u128> = Vec::new();

                for _ in 0..50 {
                    let start_time = benchmark_api::current_time();
                    block();
                    let end_time = benchmark_api::current_time();
                    let elasped = end_time - start_time;
                    elapses.push(elasped);
                }

                self.results.push(BenchResult {
                    method: name.as_bytes().to_vec(),
                    elapses,
                });
            }
        }

        $crate::sp_core::wasm_export_functions! {
            fn run_benchmarks() -> Bencher {
                let mut bencher = Bencher::default();
                $($method(&mut bencher);)+
                bencher
            }
        }
    }
}
