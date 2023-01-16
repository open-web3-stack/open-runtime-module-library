use crate::{
	colorize::{cyan, green_bold},
	BenchResult,
};
use codec::Decode;
use frame_support::traits::StorageInfo;
use linregress::{FormulaRegressionBuilder, RegressionDataBuilder};
use serde::{Deserialize, Serialize};
use sp_core::hexdisplay::HexDisplay;
use std::io::Write;
use std::time::Duration;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
struct BenchData {
	pub name: String,
	pub weight: u64,
	pub reads: u32,
	pub writes: u32,
	pub comments: Vec<String>,
}

/// Handle bench results
pub fn handle(output: Vec<u8>, storage_infos: Vec<StorageInfo>) {
	println!();

	let pkg_name = std::env::var("CARGO_PKG_NAME").unwrap_or_default().replace('-', "_");

	let results = <Vec<BenchResult> as Decode>::decode(&mut &output[..]).unwrap();
	let data: Vec<BenchData> = results
		.into_iter()
		.map(|result| {
			let name = String::from_utf8_lossy(&result.method).to_string();

			let y: Vec<f64> = result.elapses.into_iter().map(|x| x as f64).collect();
			let x: Vec<f64> = (0..y.len()).into_iter().map(|x| x as f64).collect();
			let data = vec![("Y", y), ("X", x)];
			let data = RegressionDataBuilder::new().build_from(data).unwrap();
			let formula = "Y ~ X";

			let model = FormulaRegressionBuilder::new()
				.data(&data)
				.formula(formula)
				.fit()
				.unwrap();

			let mut total_reads = 0u32;
			let mut total_writes = 0u32;
			let mut comments = Vec::<String>::new();
			let keys = <Vec<(Vec<u8>, u32, u32)> as Decode>::decode(&mut &result.keys[..]).unwrap();
			keys.into_iter().for_each(|(prefix, reads, writes)| {
				total_reads += reads;
				total_writes += writes;
				if let Some(info) = storage_infos.iter().find(|x| x.prefix.eq(&prefix)) {
					let pallet = String::from_utf8(info.pallet_name.clone()).unwrap();
					let name = String::from_utf8(info.storage_name.clone()).unwrap();
					comments.push(format!("{}::{} (r: {}, w: {})", pallet, name, reads, writes));
				} else {
					comments.push(format!(
						"Unknown 0x{} (r: {}, w: {})",
						HexDisplay::from(&prefix),
						reads,
						writes
					));
				}
			});

			comments.sort();

			let intercepted_value = model.parameters()[0] as u64;

			println!(
				"{} {:<40} {:>20} storage: {:<20}",
				green_bold("Bench"),
				cyan(&name),
				green_bold(&format!("{:?}", Duration::from_nanos(intercepted_value))),
				green_bold(&format!(
					"[r: {}, w: {}]",
					&total_reads.to_string(),
					&total_writes.to_string()
				)),
			);

			BenchData {
				name,
				weight: intercepted_value * 1_000,
				reads: total_reads,
				writes: total_writes,
				comments,
			}
		})
		.collect();

	println!();

	let outdir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let json_path = format!("{}/target/{}_bench_data.json", outdir, pkg_name);
	let mut writer = std::io::BufWriter::new(std::fs::File::create(std::path::Path::new(&json_path)).unwrap());
	serde_json::to_writer_pretty(&mut writer, &data).unwrap();
	writer.write_all(b"\n").unwrap();
	writer.flush().unwrap();

	std::io::stdout().lock().write_all(json_path.as_bytes()).unwrap();
}
