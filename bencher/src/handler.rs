use crate::BenchResult;
use codec::Decode;
use linregress::{FormulaRegressionBuilder, RegressionDataBuilder};
use serde::Serialize;

#[derive(Serialize, Default, Debug, Clone)]
struct BenchData {
	pub name: String,
	pub base_weight: u64,
	pub base_reads: u32,
	pub base_writes: u32,
}

#[derive(Serialize, Default, Debug, Clone)]
struct TemplateData {
	pub header: String,
	pub benchmarks: Vec<BenchData>,
}

// A Handlebars helper to add an underscore after every 3rd character,
// i.e. a separator for large numbers.
#[derive(Clone, Copy)]
struct UnderscoreHelper;
impl handlebars::HelperDef for UnderscoreHelper {
	fn call<'reg: 'rc, 'rc>(
		&self,
		h: &handlebars::Helper,
		_: &handlebars::Handlebars,
		_: &handlebars::Context,
		_rc: &mut handlebars::RenderContext,
		out: &mut dyn handlebars::Output,
	) -> handlebars::HelperResult {
		use handlebars::JsonRender;
		let param = h.param(0).unwrap();
		let underscore_param = underscore(param.value().render());
		out.write(&underscore_param)?;
		Ok(())
	}
}

// Add an underscore after every 3rd character, i.e. a separator for large
// numbers.
fn underscore<Number>(i: Number) -> String
where
	Number: std::string::ToString,
{
	let mut s = String::new();
	let i_str = i.to_string();
	let a = i_str.chars().rev().enumerate();
	for (idx, val) in a {
		if idx != 0 && idx % 3 == 0 {
			s.insert(0, '_');
		}
		s.insert(0, val);
	}
	s
}

// A helper to join a string of vectors.
#[derive(Clone, Copy)]
struct JoinHelper;
impl handlebars::HelperDef for JoinHelper {
	fn call<'reg: 'rc, 'rc>(
		&self,
		h: &handlebars::Helper,
		_: &handlebars::Handlebars,
		_: &handlebars::Context,
		_rc: &mut handlebars::RenderContext,
		out: &mut dyn handlebars::Output,
	) -> handlebars::HelperResult {
		use handlebars::JsonRender;
		let param = h.param(0).unwrap();
		let value = param.value();
		let joined = if value.is_array() {
			value
				.as_array()
				.unwrap()
				.iter()
				.map(|v| v.render())
				.collect::<Vec<String>>()
				.join(" ")
		} else {
			value.render()
		};
		out.write(&joined)?;
		Ok(())
	}
}

fn write(benchmarks: Vec<BenchData>) {
	// New Handlebars instance with helpers.
	let mut handlebars = handlebars::Handlebars::new();
	handlebars.register_helper("underscore", Box::new(UnderscoreHelper));
	handlebars.register_helper("join", Box::new(JoinHelper));
	// Don't HTML escape any characters.
	handlebars.register_escape_fn(|s| -> String { s.to_string() });

	let hbs_data = TemplateData {
		header: "".to_string(), // TODO: header from provided file path
		benchmarks,
	};

	const TEMPLATE: &str = include_str!("./template.hbs");
	// TODO: add option to provide custom template

	// TODO: add option to change output file path
	let mut output_path = ::std::env::current_dir().unwrap();
	output_path.push("src/module_weights.rs");

	let mut output_file = ::std::fs::File::create(output_path).unwrap();

	handlebars
		.render_template_to_write(&TEMPLATE, &hbs_data, &mut output_file)
		.unwrap();
}

/// Handle bench results
pub fn handle(output: Vec<u8>) {
	let results = <Vec<BenchResult> as Decode>::decode(&mut &output[..]).unwrap();
	let data = results
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

	// TODO: check if should write weights file
	write(data);
}
