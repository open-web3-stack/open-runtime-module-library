/// Environment variable to disable color output of the wasm build.
const WASM_BUILD_NO_COLOR: &str = "WASM_BUILD_NO_COLOR";

/// Returns `true` when color output is enabled.
pub fn color_output_enabled() -> bool {
	std::env::var(WASM_BUILD_NO_COLOR).is_err()
}

pub fn red_bold(message: &str) -> String {
	if color_output_enabled() {
		ansi_term::Color::Red.bold().paint(message).to_string()
	} else {
		message.into()
	}
}

pub fn yellow_bold(message: &str) -> String {
	if color_output_enabled() {
		ansi_term::Color::Yellow.bold().paint(message).to_string()
	} else {
		message.into()
	}
}

pub fn cyan(message: &str) -> String {
	if crate::build_wasm::color_output_enabled() {
		ansi_term::Color::Cyan.paint(message).to_string()
	} else {
		message.into()
	}
}

pub fn green_bold(message: &str) -> String {
	if crate::build_wasm::color_output_enabled() {
		ansi_term::Color::Green.bold().paint(message).to_string()
	} else {
		message.into()
	}
}
