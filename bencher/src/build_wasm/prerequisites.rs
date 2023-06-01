// This file is part of Substrate.

// Copyright (C) 2019-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::colorize::{color_output_enabled, red_bold, yellow_bold};
use std::{
	env, fs,
	io::BufRead,
	path::{Path, PathBuf},
	process::Command,
};

/// Environment variable to set the toolchain used to compile the wasm binary.
pub const WASM_BUILD_TOOLCHAIN: &str = "WASM_BUILD_TOOLCHAIN";

/// Write to the given `file` if the `content` is different.
pub fn write_file_if_changed(file: impl AsRef<Path>, content: impl AsRef<str>) {
	if fs::read_to_string(file.as_ref()).ok().as_deref() != Some(content.as_ref()) {
		fs::write(file.as_ref(), content.as_ref())
			.unwrap_or_else(|_| panic!("Writing `{}` can not fail!", file.as_ref().display()));
	}
}

/// Copy `src` to `dst` if the `dst` does not exist or is different.
pub fn copy_file_if_changed(src: PathBuf, dst: PathBuf) {
	let src_file = fs::read_to_string(&src).ok();
	let dst_file = fs::read_to_string(&dst).ok();

	if src_file != dst_file {
		fs::copy(&src, &dst)
			.unwrap_or_else(|_| panic!("Copying `{}` to `{}` can not fail; qed", src.display(), dst.display()));
	}
}

/// Get a cargo command that compiles with nightly
fn get_nightly_cargo() -> CargoCommand {
	let env_cargo = CargoCommand::new(&env::var("CARGO").expect("`CARGO` env variable is always set by cargo"));
	let default_cargo = CargoCommand::new("cargo");
	let rustup_run_nightly = CargoCommand::new_with_args("rustup", &["run", "nightly", "cargo"]);
	let wasm_toolchain = env::var(WASM_BUILD_TOOLCHAIN).ok();

	// First check if the user requested a specific toolchain
	if let Some(cmd) = wasm_toolchain.and_then(|t| get_rustup_nightly(Some(t))) {
		cmd
	} else if env_cargo.is_nightly() {
		env_cargo
	} else if default_cargo.is_nightly() {
		default_cargo
	} else if rustup_run_nightly.is_nightly() {
		rustup_run_nightly
	} else {
		// If no command before provided us with a nightly compiler, we try to search
		// one with rustup. If that fails as well, we return the default cargo and let
		// the prequisities check fail.
		get_rustup_nightly(None).unwrap_or(default_cargo)
	}
}

/// Get a nightly from rustup. If `selected` is `Some(_)`, a `CargoCommand`
/// using the given nightly is returned.
fn get_rustup_nightly(selected: Option<String>) -> Option<CargoCommand> {
	let host = format!("-{}", env::var("HOST").expect("`HOST` is always set by cargo"));

	let version = match selected {
		Some(selected) => selected,
		None => {
			let output = Command::new("rustup").args(["toolchain", "list"]).output().ok()?.stdout;
			let lines = output.as_slice().lines();

			let mut latest_nightly = None;
			for line in lines.filter_map(|l| l.ok()) {
				if line.starts_with("nightly-") && line.ends_with(&host) {
					// Rustup prints them sorted
					latest_nightly = Some(line.clone());
				}
			}

			latest_nightly?.trim_end_matches(&host).into()
		}
	};

	Some(CargoCommand::new_with_args("rustup", &["run", &version, "cargo"]))
}

/// Wraps a specific command which represents a cargo invocation.
#[derive(Debug)]
pub struct CargoCommand {
	program: String,
	args: Vec<String>,
}

impl CargoCommand {
	fn new(program: &str) -> Self {
		CargoCommand {
			program: program.into(),
			args: Vec::new(),
		}
	}

	fn new_with_args(program: &str, args: &[&str]) -> Self {
		CargoCommand {
			program: program.into(),
			args: args.iter().map(ToString::to_string).collect(),
		}
	}

	pub fn command(&self) -> Command {
		let mut cmd = Command::new(&self.program);
		cmd.args(&self.args);
		cmd
	}

	/// Check if the supplied cargo command is a nightly version
	fn is_nightly(&self) -> bool {
		// `RUSTC_BOOTSTRAP` tells a stable compiler to behave like a nightly. So, when
		// this env variable is set, we can assume that whatever rust compiler we have,
		// it is a nightly compiler. For "more" information, see:
		// https://github.com/rust-lang/rust/blob/fa0f7d0080d8e7e9eb20aa9cbf8013f96c81287f/src/libsyntax/feature_gate/check.rs#L891
		env::var("RUSTC_BOOTSTRAP").is_ok()
			|| self
				.command()
				.arg("--version")
				.output()
				.map_err(|_| ())
				.and_then(|o| String::from_utf8(o.stdout).map_err(|_| ()))
				.unwrap_or_default()
				.contains("-nightly")
	}
}

/// Wraps a [`CargoCommand`] and the version of `rustc` the cargo command uses.
pub struct CargoCommandVersioned {
	command: CargoCommand,
	version: String,
}

impl CargoCommandVersioned {
	fn new(command: CargoCommand, version: String) -> Self {
		Self { command, version }
	}

	/// Returns the `rustc` version.
	pub fn rustc_version(&self) -> &str {
		&self.version
	}
}

impl std::ops::Deref for CargoCommandVersioned {
	type Target = CargoCommand;

	fn deref(&self) -> &CargoCommand {
		&self.command
	}
}

use tempfile::tempdir;

/// Checks that all prerequisites are installed.
///
/// Returns the versioned cargo command on success.
pub fn check() -> Result<CargoCommandVersioned, String> {
	let cargo_command = get_nightly_cargo();

	if !cargo_command.is_nightly() {
		return Err(red_bold("Rust nightly not installed, please install it!"));
	}

	check_wasm_toolchain_installed(cargo_command)
}

/// Create the project that will be used to check that the wasm toolchain is
/// installed and to extract the rustc version.
fn create_check_toolchain_project(project_dir: &Path) {
	let lib_rs_file = project_dir.join("src/lib.rs");
	let main_rs_file = project_dir.join("src/main.rs");
	let build_rs_file = project_dir.join("build.rs");
	let manifest_path = project_dir.join("Cargo.toml");

	write_file_if_changed(
		manifest_path,
		r#"
			[package]
			name = "wasm-test"
			version = "1.0.0"
			edition = "2021"
			build = "build.rs"

			[lib]
			name = "wasm_test"
			crate-type = ["cdylib"]

			[workspace]
		"#,
	);
	write_file_if_changed(lib_rs_file, "pub fn test() {}");

	// We want to know the rustc version of the rustc that is being used by our
	// cargo command. The cargo command is determined by some *very* complex
	// algorithm to find the cargo command that supports nightly.
	// The best solution would be if there is a `cargo rustc --version` command,
	// which sadly doesn't exists. So, the only available way of getting the rustc
	// version is to build a project and capture the rustc version in this build
	// process. This `build.rs` is exactly doing this. It gets the rustc version by
	// calling `rustc --version` and exposing it in the `RUSTC_VERSION` environment
	// variable.
	write_file_if_changed(
		build_rs_file,
		r#"
			fn main() {
				let rustc_cmd = std::env::var("RUSTC").ok().unwrap_or_else(|| "rustc".into());

				let rustc_version = std::process::Command::new(rustc_cmd)
					.arg("--version")
					.output()
					.ok()
					.and_then(|o| String::from_utf8(o.stdout).ok());

				println!(
					"cargo:rustc-env=RUSTC_VERSION={}",
					rustc_version.unwrap_or_else(|| "unknown rustc version".into()),
				);
			}
		"#,
	);
	// Just prints the `RURSTC_VERSION` environment variable that is being created
	// by the `build.rs` script.
	write_file_if_changed(
		main_rs_file,
		r#"
			fn main() {
				println!("{}", env!("RUSTC_VERSION"));
			}
		"#,
	);
}

fn check_wasm_toolchain_installed(cargo_command: CargoCommand) -> Result<CargoCommandVersioned, String> {
	let temp = tempdir().expect("Creating temp dir does not fail; qed");
	fs::create_dir_all(temp.path().join("src")).expect("Creating src dir does not fail; qed");
	create_check_toolchain_project(temp.path());

	let err_msg = red_bold("Rust WASM toolchain not installed, please install it!");
	let manifest_path = temp.path().join("Cargo.toml").display().to_string();

	let mut build_cmd = cargo_command.command();
	build_cmd.args([
		"build",
		"--target=wasm32-unknown-unknown",
		"--manifest-path",
		&manifest_path,
	]);

	if color_output_enabled() {
		build_cmd.arg("--color=always");
	}

	let mut run_cmd = cargo_command.command();
	run_cmd.args(["run", "--manifest-path", &manifest_path]);

	build_cmd.output().map_err(|_| err_msg.clone()).and_then(|s| {
		if s.status.success() {
			let version = run_cmd.output().ok().and_then(|o| String::from_utf8(o.stdout).ok());
			Ok(CargoCommandVersioned::new(
				cargo_command,
				version.unwrap_or_else(|| "unknown rustc version".into()),
			))
		} else {
			match String::from_utf8(s.stderr) {
				Ok(ref err) if err.contains("linker `rust-lld` not found") => {
					Err(red_bold("`rust-lld` not found, please install it!"))
				}
				Ok(ref err) => Err(format!(
					"{}\n\n{}\n{}\n{}{}\n",
					err_msg,
					yellow_bold("Further error information:"),
					yellow_bold(&"-".repeat(60)),
					err,
					yellow_bold(&"-".repeat(60)),
				)),
				Err(_) => Err(err_msg),
			}
		}
	})
}
