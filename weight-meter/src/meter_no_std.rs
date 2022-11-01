// TODO: research if there's a better way
#![cfg(not(feature = "std"))]

use super::{Meter, Weight};

static mut METER: Meter = Meter {
	used_weight: Weight::zero(),
	depth: 0,
};

pub fn start(weight: Weight) {
	unsafe {
		if METER.depth == 0 {
			METER.used_weight = weight;
		}
		METER.depth = METER.depth.saturating_add(1);
	}
}

pub fn using(weight: Weight) {
	unsafe {
		METER.used_weight = METER.used_weight.saturating_add(weight);
	}
}

pub fn finish() {
	unsafe {
		METER.depth.checked_sub(1).map_or_else(
			|| {
				debug_assert!(false);
				0
			},
			|v| v,
		);
	}
}

pub fn used_weight() -> Weight {
	unsafe { METER.used_weight }
}
