// TODO: research if there's a better way
#![cfg(feature = "std")]

use super::{Meter, Weight};
use std::cell::RefCell;

thread_local! {
	static METER: RefCell<Meter> = RefCell::new(Meter {
		used_weight: 0,
		depth: 0,
	});
}

/// Start weight meter with base weight
pub fn start() {
	METER.with(|v| {
		let mut meter = v.borrow_mut();
		if meter.depth == 0 {
			meter.used_weight = 0;
		}
		meter.depth = meter.depth.saturating_add(1);
	});
}

/// Increment used weight
pub fn using(weight: Weight) {
	METER.with(|v| {
		let mut meter = v.borrow_mut();
		meter.used_weight = meter.used_weight.saturating_add(weight);
	})
}

/// Finish weight meter
pub fn finish() {
	METER.with(|v| {
		let mut meter = v.borrow_mut();
		meter.depth = meter.depth.saturating_sub(1);
	})
}

/// Get used weight
pub fn used_weight() -> Weight {
	METER.with(|v| v.borrow().used_weight)
}
