use rand::{distributions::Alphanumeric, thread_rng, Rng};

pub mod prerequisites;
pub mod wasm_project;

/// Environment variable to disable color output of the wasm build.
const WASM_BUILD_NO_COLOR: &str = "WASM_BUILD_NO_COLOR";

/// Returns `true` when color output is enabled.
pub fn color_output_enabled() -> bool {
	std::env::var(WASM_BUILD_NO_COLOR).is_err()
}

pub fn build() -> std::io::Result<Vec<u8>> {
	let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
	let pkg_name = std::env::var("CARGO_PKG_NAME").unwrap();

	let random = thread_rng()
		.sample_iter(&Alphanumeric)
		.take(16)
		.map(char::from)
		.collect::<String>();

	let mut out_dir = std::path::PathBuf::from(manifest_dir);
	out_dir.push(format!("target/release/build/{}-{}/out", pkg_name, random));

	std::env::set_var("OUT_DIR", out_dir.display().to_string());

	let mut project_cargo_toml = std::env::current_dir()?;
	project_cargo_toml.push("Cargo.toml");

	let default_rustflags = "-Clink-arg=--export=__heap_base -C link-arg=--import-memory";
	let cargo_cmd = match prerequisites::check() {
		Ok(cmd) => cmd,
		Err(err_msg) => {
			eprintln!("{}", err_msg);
			std::process::exit(1);
		}
	};

	let (wasm_binary, bloaty) = wasm_project::create_and_compile(
		&project_cargo_toml,
		default_rustflags,
		cargo_cmd,
		vec!["bench".to_string()],
		None,
	);

	let (wasm_binary, _wasm_binary_bloaty) = if let Some(wasm_binary) = wasm_binary {
		(
			wasm_binary.wasm_binary_path_escaped(),
			bloaty.wasm_binary_bloaty_path_escaped(),
		)
	} else {
		(
			bloaty.wasm_binary_bloaty_path_escaped(),
			bloaty.wasm_binary_bloaty_path_escaped(),
		)
	};

	let bytes = std::fs::read(wasm_binary)?;

	Ok(bytes.to_vec())
}
