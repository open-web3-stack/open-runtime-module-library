use super::{
	ext::BenchExt,
	tracker::{BenchTracker, BenchTrackerExt},
};
use frame_benchmarking::frame_support::sp_runtime::traits::Block;
use sc_executor::{sp_wasm_interface::HostFunctions, WasmExecutionMethod, WasmExecutor};
use sc_executor_common::runtime_blob::RuntimeBlob;
use sp_externalities::Extensions;
use sp_state_machine::{Ext, OverlayedChanges, StorageTransactionCache};
use sp_std::sync::Arc;

/// Run benches
pub fn run<B: Block>(wasm_code: Vec<u8>) -> std::result::Result<Vec<u8>, String> {
	let mut overlay = OverlayedChanges::default();
	let mut cache = StorageTransactionCache::default();
	let state = sc_client_db::BenchmarkingState::<B>::new(Default::default(), Default::default(), false, true).unwrap();

	let tracker = Arc::new(BenchTracker::new());
	let tracker_ext = BenchTrackerExt(Arc::clone(&tracker));

	let mut extensions = Extensions::default();
	extensions.register(tracker_ext);

	let ext = Ext::<_, _>::new(&mut overlay, &mut cache, &state, Some(&mut extensions));
	let mut bench_ext = BenchExt::new(ext, tracker);

	let mut host_functions = sp_io::SubstrateHostFunctions::host_functions();
	host_functions.append(&mut frame_benchmarking::benchmarking::HostFunctions::host_functions());
	host_functions.append(&mut super::bench::HostFunctions::host_functions());

	let executor = WasmExecutor::new(
		WasmExecutionMethod::Compiled,
		Default::default(),
		host_functions,
		1,
		None,
	);

	let blob = RuntimeBlob::uncompress_if_needed(&wasm_code[..]).unwrap();

	executor.uncached_call(blob, &mut bench_ext, false, "run_benches", &[])
}
