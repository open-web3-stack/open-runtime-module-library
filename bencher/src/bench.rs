/// Define benches
///
/// Create a file `src/benches.rs`:
/// ```.ignore
/// #![cfg_attr(not(feature = "std"), no_std)]
/// #![allow(dead_code)]
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
        use $crate::sp_std::prelude::*;
        #[$crate::sp_runtime_interface::runtime_interface]
        pub trait BenchApi {
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
                    let start_time = bench_api::current_time();
                    block();
                    let end_time = bench_api::current_time();
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
            fn run_benches() -> Bencher {
                let mut bencher = Bencher::default();
                $($method(&mut bencher);)+
                bencher
            }
        }
    }
}
