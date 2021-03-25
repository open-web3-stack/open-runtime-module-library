use platforms::*;
use std::process::Command;

/// Generate the `cargo:` key output
pub fn generate_cargo_keys() {
	println!(
		"cargo:rustc-env=SUBSTRATE_CLI_IMPL_VERSION={}",
		get_version(get_commit())
	)
}

fn get_platform() -> String {
	let env_dash = if TARGET_ENV.is_some() { "-" } else { "" };

	format!(
		"{}-{}{}{}",
		TARGET_ARCH.as_str(),
		TARGET_OS.as_str(),
		env_dash,
		TARGET_ENV.map(|x| x.as_str()).unwrap_or(""),
	)
}

fn get_version(impl_commit: String) -> String {
	let commit_dash = if impl_commit.is_empty() { "" } else { "-" };

	format!(
		"{}{}{}-{}",
		std::env::var("CARGO_PKG_VERSION").unwrap_or_default(),
		commit_dash,
		impl_commit,
		get_platform(),
	)
}

fn get_commit() -> String {
	let commit = std::env::var("GIT_COMMIT").unwrap_or_default();
	if !commit.is_empty() {
		return commit;
	}

	let output = Command::new("git").args(&["rev-parse", "--short", "HEAD"]).output();

	match output {
		Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().into(),
		Ok(o) => {
			println!("cargo:warning=Git command failed with status: {}", o.status);
			"unknown".into()
		}
		Err(err) => {
			println!("cargo:warning=Failed to execute git command: {}", err);
			"unknown".into()
		}
	}
}
