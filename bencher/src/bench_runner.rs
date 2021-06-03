use frame_benchmarking::frame_support::sp_runtime::traits::{Block, NumberFor};
use sc_executor::{sp_wasm_interface::HostFunctions, WasmExecutionMethod, WasmExecutor};
use sc_executor_common::runtime_blob::RuntimeBlob;
use sp_state_machine::{Ext, OverlayedChanges, StorageTransactionCache};

/// Run benches
pub fn run<B: Block>(wasm_code: Vec<u8>) -> std::result::Result<Vec<u8>, String> {
	let mut overlay = OverlayedChanges::default();
	let mut cache = StorageTransactionCache::default();
	let state = sc_client_db::BenchmarkingState::<B>::new(Default::default(), Default::default(), false).unwrap();
	let mut ext = Ext::<_, NumberFor<B>, _>::new(&mut overlay, &mut cache, &state, None, None);

	let mut host_functions = sp_io::SubstrateHostFunctions::host_functions();
	host_functions.append(&mut frame_benchmarking::benchmarking::HostFunctions::host_functions());
	host_functions.append(&mut super::bencher::HostFunctions::host_functions());

	let executor = WasmExecutor::new(
		WasmExecutionMethod::Compiled,
		Default::default(),
		host_functions,
		1,
		None,
	);

	let blob = RuntimeBlob::uncompress_if_needed(&wasm_code[..]).unwrap();

	executor.uncached_call(blob, &mut ext, true, "run_benches", &[])
}
