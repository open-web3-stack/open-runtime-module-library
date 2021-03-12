// TODO: research if there's a better way
#![cfg_attr(not(feature = "std"), no_std)]

pub use weight_meter_procedural::*;

static mut USED_WEIGHT: u64 = 0;
static mut NESTED: u64 = 0;

pub fn using(m: u64) {
	unsafe {
		USED_WEIGHT = USED_WEIGHT.saturating_add(m);
	}
}

pub fn used_weight() -> u64 {
	unsafe { USED_WEIGHT }
}

pub fn start_with(m: u64) {
	unsafe {
		if NESTED == 0 {
			USED_WEIGHT = m;
		}
		NESTED = NESTED.saturating_add(1);
	}
}

pub fn end() {
	unsafe {
		NESTED = NESTED.saturating_sub(1);
	}
}
