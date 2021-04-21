use frame_benchmarking::{
	benchmarking,
	frame_support::sp_runtime::traits::{Block, NumberFor},
};
use sc_client_db::BenchmarkingState;
use sc_executor::{sp_wasm_interface::HostFunctions, WasmExecutionMethod, WasmExecutor};
use sp_core::traits::{CallInWasm, MissingHostFunctions};
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
		.call_in_wasm(
			&wasm_code[..],
			None,
			"run_benches",
			&[],
			&mut ext,
			MissingHostFunctions::Disallow,
		)
		.unwrap()
}
