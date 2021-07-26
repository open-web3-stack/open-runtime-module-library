use crate::{
	colorize::{cyan, green_bold},
	BenchResult,
};
use codec::Decode;
use linregress::{FormulaRegressionBuilder, RegressionDataBuilder};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::time::Duration;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
struct BenchData {
	pub name: String,
	pub weight: u64,
	pub reads: u32,
	pub writes: u32,
}

/// Handle bench results
pub fn handle(output: Vec<u8>) {
	println!();

	let pkg_name = std::env::var("CARGO_PKG_NAME").unwrap_or_default().replace("-", "_");

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

			println!(
				"{} {:<60} {:>20}  {:<20}  {:<20}",
				green_bold("Bench"),
				cyan(&format!("{}::{}", pkg_name, name)),
				green_bold(&format!(
					"{:?}",
					Duration::from_nanos(model.parameters.intercept_value as u64)
				)),
				format!("reads: {}", green_bold(&result.reads.to_string())),
				format!("writes: {}", green_bold(&result.writes.to_string()))
			);

			BenchData {
				name,
				weight: model.parameters.intercept_value as u64 * 1_000,
				reads: result.reads,
				writes: result.writes,
			}
		})
		.collect();

	println!();

	if let Ok(json) = serde_json::to_string(&data) {
		let stdout = ::std::io::stdout();
		let mut handle = stdout.lock();

		handle.write_all(&json.as_bytes()).unwrap();
	} else {
		eprintln!("Could not write benchdata to JSON");
	}
}
