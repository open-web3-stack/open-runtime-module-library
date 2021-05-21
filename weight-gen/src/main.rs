use clap::{AppSettings, Clap};
use serde::{Deserialize, Serialize};
use std::io::Read;

#[derive(Clap)]
#[clap(version = "01.0", author = "Laminar Developers <hello@laminar.one>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
	input: Option<String>,
	#[clap(short, long)]
	template: Option<String>,
	#[clap(short, long)]
	header: Option<String>,
	#[clap(short, long)]
	out: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct BenchData {
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
		let param = h.param(0).expect("Unable to retrieve param from handlebars helper");
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
		let param = h.param(0).expect("Unable to retrieve param from handlebars helper");
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

fn parse_stdio() -> Option<Vec<BenchData>> {
	let mut buffer = String::new();
	let stdin = std::io::stdin();
	let mut handle = stdin.lock();

	handle.read_to_string(&mut buffer).expect("Unable to read from stdin");

	let lines: Vec<&str> = buffer.split('\n').collect();
	for line in lines {
		let json = serde_json::from_str(line);

		if let Ok(data) = json {
			return Some(data);
		}
	}

	None
}

fn main() {
	let opts: Opts = Opts::parse();

	let benchmarks: Vec<BenchData> = {
		if let Some(data) = opts.input {
			serde_json::from_str(&data).expect("Could not parse JSON data")
		} else {
			parse_stdio().expect("Could not parse JSON data")
		}
	};

	let mut handlebars = handlebars::Handlebars::new();
	handlebars.register_helper("underscore", Box::new(UnderscoreHelper));
	handlebars.register_helper("join", Box::new(JoinHelper));
	// Don't HTML escape any characters.
	handlebars.register_escape_fn(|s| -> String { s.to_string() });

	// Use empty header if a header path is not given.
	let header = {
		if let Some(path) = opts.header {
			::std::fs::read_to_string(&path).expect("Header file not found")
		} else {
			String::from("")
		}
	};

	let hbs_data = TemplateData { header, benchmarks };

	const DEFAULT_TEMPLATE: &str = include_str!("./template.hbs");

	// Use default template if template path is not given.
	let template = {
		if let Some(path) = opts.template {
			::std::fs::read_to_string(&path).expect("Template file not found")
		} else {
			String::from(DEFAULT_TEMPLATE)
		}
	};

	// Write benchmark to file or print to terminal if output path is not given.
	if let Some(path) = opts.out {
		let mut output_file = ::std::fs::File::create(&path).expect("Could not create output file");

		handlebars
			.render_template_to_write(&template, &hbs_data, &mut output_file)
			.expect("Unable to render template");
	} else {
		let template_string = handlebars
			.render_template(&template, &hbs_data)
			.expect("Unable to render template");

		println!("{}", template_string);
	}
}
