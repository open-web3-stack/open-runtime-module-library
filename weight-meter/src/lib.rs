#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::weights::Weight;

struct Meter {
	used_weight: Weight,
	// Deep gets incremented when entering call or a sub-call
	// This is used to avoid miscalculation during sub-calls
	deep: usize,
}

mod meter_no_std;
mod meter_std;

#[cfg(feature = "std")]
pub use meter_std::*;

#[cfg(not(feature = "std"))]
pub use meter_no_std::*;

pub use weight_meter_procedural::*;
