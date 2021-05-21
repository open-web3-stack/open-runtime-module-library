use frame_benchmarking::{
	benchmarking,
	frame_support::sp_runtime::traits::{Block, NumberFor},
};
use sc_client_db::BenchmarkingState;
use sc_executor::{sp_wasm_interface::HostFunctions, WasmExecutionMethod, WasmExecutor};
use sc_executor_common::runtime_blob::RuntimeBlob;
use sp_io::SubstrateHostFunctions;
use sp_state_machine::{Ext, OverlayedChanges, StorageTransactionCache};

/// Run benches
pub fn run<B: Block>(wasm_code: Vec<u8>) -> Vec<u8> {
	let mut overlay = OverlayedChanges::default();
	let mut cache = StorageTransactionCache::default();
	let state = BenchmarkingState::<B>::new(Default::default(), Default::default(), false).unwrap();
	let mut ext = Ext::<_, NumberFor<B>, _>::new(&mut overlay, &mut cache, &state, None, None);

	let mut host_functions = benchmarking::HostFunctions::host_functions();
	host_functions.append(&mut SubstrateHostFunctions::host_functions());

	let executor = WasmExecutor::new(
		WasmExecutionMethod::Compiled,
		Default::default(),
		host_functions,
		1,
		None,
	);

	executor
		.uncached_call(
			RuntimeBlob::uncompress_if_needed(&wasm_code[..]).unwrap(),
			&mut ext,
			true,
			"run_benches",
			&[],
		)
		.unwrap()
}
