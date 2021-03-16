// TODO: research if there's a better way
#![cfg(feature = "std")]

use super::{Meter, Weight};

static METER: spin::Mutex<Meter> = spin::Mutex::new(Meter {
	used_weight: 0,
	depth: 0,
});

pub fn start_with(base: Weight) {
	let mut meter = METER.lock();
	if meter.depth == 0 {
		meter.used_weight = base;
	}
	meter.depth = meter.depth.saturating_add(1);
	drop(meter);
}

pub fn using(weight: Weight) {
	let mut meter = METER.lock();
	meter.used_weight = meter.used_weight.saturating_add(weight);
	drop(meter);
}

pub fn finish() {
	let mut meter = METER.lock();
	meter.depth = meter.depth.saturating_sub(1);
	drop(meter);
}

pub fn used_weight() -> Weight {
	let meter = METER.lock();
	let used_weight = meter.used_weight;
	drop(meter);
	used_weight
}
