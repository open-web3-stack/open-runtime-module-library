use crate::{
	colorize::{cyan, green_bold, yellow_bold},
	tracker::Warning,
	Bencher,
};
use codec::Decode;
use frame_support::traits::StorageInfo;
use linregress::{FormulaRegressionBuilder, RegressionDataBuilder};
use serde::{Deserialize, Serialize};
use sp_core::hexdisplay::HexDisplay;
use std::{io::Write, string::String, time::Duration};

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct BenchData {
	pub name: String,
	pub time: Duration,
	pub reads: u32,
	pub writes: u32,
	pub keys: Vec<(Vec<u8>, u32, u32)>,
	pub warnings: Vec<Warning>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct BenchDataOutput {
	pub name: String,
	pub weight: u64,
	pub reads: u32,
	pub writes: u32,
	pub comments: Vec<String>,
}

/// Handle bench results
pub fn parse(output: Vec<u8>) -> BenchData {
	let bencher = <Bencher as Decode>::decode(&mut &output[..]).unwrap();
	let warnings = <Vec<Warning> as Decode>::decode(&mut &bencher.warnings[..]).unwrap();
	let y: Vec<f64> = bencher.elapses.into_iter().map(|x| x as f64).collect();
	let x: Vec<f64> = (0..y.len()).map(|x| x as f64).collect();
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
	let keys = <Vec<(Vec<u8>, u32, u32)> as Decode>::decode(&mut &bencher.keys[..]).unwrap();

	keys.iter().for_each(|(_prefix, reads, writes)| {
		total_reads += reads;
		total_writes += writes;
	});

	let intercepted_value = model.parameters()[0] as u64;

	let time = Duration::from_nanos(intercepted_value);

	BenchData {
		name: String::from_utf8_lossy(&bencher.method).to_string(),
		time,
		reads: total_reads,
		writes: total_writes,
		keys,
		warnings,
	}
}

fn get_package_name() -> String {
	std::env::var("CARGO_PKG_NAME").unwrap_or_default()
}

pub fn print_start(method: &str) {
	print!("{} {:<60}...", green_bold("Bench"), cyan(method));
	std::io::stdout().flush().unwrap();
}
pub fn print_summary(data: &BenchData) {
	print!(
		"\r{} {:<60} {:>20} storage: {:<20}\n",
		green_bold("Bench"),
		cyan(&data.name),
		green_bold(&format!("{:?}", data.time)),
		green_bold(&format!(
			"[r: {:>2}, w: {:>2}]",
			data.reads.to_string(),
			data.writes.to_string()
		)),
	);

	for warning in &data.warnings {
		println!("{} {}", yellow_bold("WARNING:"), yellow_bold(&warning.to_string()));
	}
}

pub fn save_output_json(data: Vec<BenchData>, storage_infos: Vec<StorageInfo>) {
	let data = data
		.into_iter()
		.map(|x| {
			let mut comments: Vec<String> = x
				.keys
				.into_iter()
				.map(|(prefix, reads, writes)| {
					if let Some(info) = storage_infos.iter().find(|x| x.prefix.eq(&prefix)) {
						let pallet = String::from_utf8(info.pallet_name.clone()).unwrap();
						let name = String::from_utf8(info.storage_name.clone()).unwrap();
						format!("{}::{} (r: {}, w: {})", pallet, name, reads, writes)
					} else {
						format!("Unknown 0x{} (r: {}, w: {})", HexDisplay::from(&prefix), reads, writes)
					}
				})
				.collect();

			comments.sort();

			BenchDataOutput {
				name: x.name,
				weight: x.time.as_nanos() as u64 * 1_000,
				reads: x.reads,
				writes: x.writes,
				comments,
			}
		})
		.collect::<Vec<BenchDataOutput>>();

	let outdir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let pkg_name = get_package_name().replace('-', "_");
	let json_path = format!("{outdir}/target/{pkg_name}_bench_data.json");
	let mut writer = std::io::BufWriter::new(std::fs::File::create(std::path::Path::new(&json_path)).unwrap());
	serde_json::to_writer_pretty(&mut writer, &data).unwrap();
	writer.write_all(b"\n").unwrap();
	writer.flush().unwrap();

	println!("\nOutput JSON file:\n{}", json_path);
}
