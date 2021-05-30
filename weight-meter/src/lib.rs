#![cfg_attr(not(feature = "std"), no_std)]

//! 1. Add macro attribute on method you want to benchmark.
//! ```ignore
//! #[orml_weight_meter::weight(0)]
//! fn inner_do_something(something: u32) {
//!     // Update storage.
//!     Something::<T>::put(something);
//! }
//! ```
//! Start with `0` and after the weights is generated then it can be replaced
//! with generated weight. Macro will inject callable methods that wraps inner
//! methods. Generated call will start with prefix `method_` followed by method
//! name. This only works for methods with `orml_weight_meter::weight` attribute
//! and only when running benchmarks.
//!
//! 2. Create benchmarks using orml_bencher and generate the weights with
//! orml_weight_gen
//! After running the benchmarks and the weights have been generated then we can
//! replace
//! ```ignore
//! #[orml_weight_meter::weight(0)]
//! ```
//! with
//!```ignore
//! #[orml_weight_meter::weight(T::WeightInfo::method_inner_do_something())]
//! ```
//!
//! 3. Use WeightMeter on your calls by adding macro
//! `#[orml_weight_meter::start]` and at the end use
//! `orml_weight_meter::used_weight()` to get used weight.
//!```ignore
//! #[pallet::call]
//! impl<T: Config> Pallet<T> {
//!     #[pallet::weight(T::WeightInfo::do_something())]
//!     #[orml_weight_meter::start]
//!     pub fn do_something(origin: OriginFor<T>, something: u32) ->
//!     DispatchResultWithPostInfo {
//!         let who = ensure_signed(origin)?;
//!         Self::inner_do_something(something);
//!         // Emit an event.
//!         Self::deposit_event(Event::SomethingStored(something, who));
//!         Ok(PostDispatchInfo::from(Some(orml_weight_meter::used_weight())))
//!     }
//! }
//! ```

use frame_support::weights::Weight;

struct Meter {
	used_weight: Weight,
	// Depth gets incremented when entering call or a sub-call
	// This is used to avoid miscalculation during sub-calls
	depth: u8,
}

mod meter_no_std;
mod meter_std;

extern crate self as orml_weight_meter;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "std")]
pub use meter_std::*;

#[cfg(not(feature = "std"))]
pub use meter_no_std::*;

/// Start weight meter
pub use weight_meter_procedural::start;

/// Measure each methods weight
pub use weight_meter_procedural::weight;
