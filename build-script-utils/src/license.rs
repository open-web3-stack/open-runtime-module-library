use std::{fs::File, io::Read, path::Path};
use walkdir::{DirEntry, WalkDir};

// Check the license text that should be present at the beginning of every
// source file.
pub fn check_file_licenses<P: AsRef<Path>>(path: P, expected_license_text: &[u8], exclude_paths: &[&str]) {
	// The following directories will be excluded from the license scan.
	let skips = ["artifacts", "corpus", "target", "fuzz_targets"];

	let path = path.as_ref();

	let mut iter = WalkDir::new(path).into_iter();
	while let Some(entry) = iter.next() {
		let entry = entry.unwrap();
		let entry_type = entry.file_type();

		// Skip the hidden entries efficiently.
		if is_hidden(&entry) {
			if entry.file_type().is_dir() {
				iter.skip_current_dir();
			}
			continue;
		}

		// Skip the specified directories and paths.
		if entry_type.is_dir()
			&& (skips.contains(&entry.file_name().to_str().unwrap_or(""))
				|| exclude_paths.contains(&entry.path().to_str().unwrap_or("")))
		{
			iter.skip_current_dir();

			continue;
		}

		// Check all files with the ".rs" extension.
		if entry_type.is_file() && entry.file_name().to_str().unwrap_or("").ends_with(".rs") {
			let file = File::open(entry.path()).unwrap();
			let mut contents = Vec::with_capacity(expected_license_text.len());
			file.take(expected_license_text.len() as u64)
				.read_to_end(&mut contents)
				.unwrap();

			assert!(
				contents == expected_license_text,
				"The license in \"{}\" is either missing or it doesn't match the expected string!",
				entry.path().display()
			);
		}
	}

	// Re-run upon any changes to the workspace.
	println!("cargo:rerun-if-changed=.");
}

// hidden files and directories efficiently.
fn is_hidden(entry: &DirEntry) -> bool {
	entry
		.file_name()
		.to_str()
		.map(|s| s.starts_with('.') && !s.starts_with(".."))
		.unwrap_or(false)
}
