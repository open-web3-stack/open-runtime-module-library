use super::{
	bench_ext::BenchExt,
	tracker::{BenchTracker, BenchTrackerExt},
};
use frame_support::sp_runtime::traits::Block;
use sc_executor::{WasmExecutionMethod, WasmExecutor, WasmtimeInstantiationStrategy};
use sc_executor_common::runtime_blob::RuntimeBlob;
use sp_externalities::Extensions;
use sp_state_machine::{Ext, OverlayedChanges, StorageTransactionCache};
use sp_std::sync::Arc;

type ComposeHostFunctions = (sp_io::SubstrateHostFunctions, super::bench::HostFunctions);

/// Run benches
pub fn run<B: Block>(wasm_code: Vec<u8>) -> std::result::Result<Vec<u8>, sc_executor_common::error::Error> {
	let mut overlay = OverlayedChanges::default();
	let mut cache = StorageTransactionCache::default();
	let state = sc_client_db::BenchmarkingState::<B>::new(Default::default(), Default::default(), false, true).unwrap();

	let tracker = Arc::new(BenchTracker::new());
	let tracker_ext = BenchTrackerExt(Arc::clone(&tracker));

	let mut extensions = Extensions::default();
	extensions.register(tracker_ext);

	let ext = Ext::<_, _>::new(&mut overlay, &mut cache, &state, Some(&mut extensions));
	let mut bench_ext = BenchExt::new(ext, tracker);

	let executor = WasmExecutor::<ComposeHostFunctions>::new(
		WasmExecutionMethod::Compiled {
			instantiation_strategy: WasmtimeInstantiationStrategy::PoolingCopyOnWrite,
		},
		Default::default(),
		1,
		None,
		1,
	);

	let blob = RuntimeBlob::uncompress_if_needed(&wasm_code[..]).unwrap();

	executor.uncached_call(blob, &mut bench_ext, false, "run_benches", &[])
}
