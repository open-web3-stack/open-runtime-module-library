// TODO: research if there's a better way
#![cfg(not(feature = "std"))]

use super::{Meter, Weight};

static mut METER: Meter = Meter {
	used_weight: 0,
	deep: 0,
};

pub fn start_with(base: Weight) {
	unsafe {
		if METER.deep == 0 {
			METER.used_weight = base;
		}
		METER.deep = METER.deep.saturating_add(1);
	}
}

pub fn using(weight: Weight) {
	unsafe {
		METER.used_weight = METER.used_weight.saturating_add(weight);
	}
}

pub fn finish() {
	unsafe {
		METER.deep = METER.deep.saturating_sub(1);
	}
}

pub fn used_weight() -> Weight {
	unsafe { METER.used_weight }
}
