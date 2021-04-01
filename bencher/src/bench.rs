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
