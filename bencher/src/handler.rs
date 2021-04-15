use crate::BenchResult;
use codec::Decode;
use linregress::{FormulaRegressionBuilder, RegressionDataBuilder};
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
struct BenchData {
	pub name: String,
	pub base_weight: u64,
	pub base_reads: u32,
	pub base_writes: u32,
}

/// Handle bench results
pub fn handle(output: Vec<u8>) {
	let results = <Vec<BenchResult> as Decode>::decode(&mut &output[..]).unwrap();
	let data: Vec<BenchData> = results
		.into_iter()
		.map(|result| {
			let name = String::from_utf8_lossy(&result.method).to_string();

			eprintln!("{:#?}", result);

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

			BenchData {
				name,
				base_weight: model.parameters.intercept_value as u64 * 1_000,
				base_reads: result.reads,
				base_writes: result.writes,
			}
		})
		.collect();

	if let Ok(json) = serde_json::to_string(&data) {
		let stdout = ::std::io::stdout();
		let mut handle = stdout.lock();

		handle.write_all(&json.as_bytes()).unwrap();
	} else {
		eprintln!("Could not write benchdata to JSON");
	}
}
