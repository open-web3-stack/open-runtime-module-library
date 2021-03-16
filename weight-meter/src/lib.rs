#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::weights::Weight;

struct Meter {
	used_weight: Weight,
	// Depth gets incremented when entering call or a sub-call
	// This is used to avoid miscalculation during sub-calls
	depth: u8,
}

mod meter_no_std;
mod meter_std;

#[cfg(feature = "std")]
pub use meter_std::*;

#[cfg(not(feature = "std"))]
pub use meter_no_std::*;

/// Start weight meter with base weight
pub use weight_meter_procedural::start_with;

/// Measure each methods weight
pub use weight_meter_procedural::weight;

/// `method_benchmarks` attribute macro let's you benchmark inner methods
///
/// 1. Add macro attribute on top of the module declaration
/// ```ignore
/// #[orml_weight_meter::method_benchmarks]
/// #[frame_support::pallet]
/// pub mod module {
///     ..
/// }
/// ```
///
/// 2. Add macro attribute on method you want to benchmark.
/// ```ignore
/// #[orml_weight_meter::weight(0)]
/// fn inner_do_something(something: u32) {
///     // Update storage.
///     Something::<T>::put(something);
/// }
/// ```
/// Start with `0` and after the weights is generated then it can be replaced
/// with generated weight. Macro will inject callable methods that wraps inner
/// methods. Generated call will start with prefix `method_` followed by method
/// name. This only works for methods with `orml_weight_meter::weight` attribute
/// and only when running benchmarks.
///
/// 3. Create benchmarks as we normally do. Just need to use prefix `method_`
/// followed by method name.
/// ```ignore
/// method_inner_do_something {
///     let caller = whitelisted_caller();
/// }: _(frame_system::RawOrigin::Signed(caller), 10)
/// ```
/// After running benchmarks and weights been generated then we can replace `
/// ```ignore
/// #[orml_weight_meter::weight(0)]
/// ```
/// with
/// ```ignore
/// #[orml_weight_meter::weight(T::WeightInfo::method_inner_do_something())]
/// ```
///
/// 4. Use WeightMeter on your calls by adding macro
/// `#[orml_weight_meter::start_with(<base>)]` and at the end use
/// `orml_weight_meter::used_weight()` to get used weight. ```ignore
/// #[pallet::call]
/// impl<T: Config> Pallet<T> {
///     #[pallet::weight(T::WeightInfo::do_something())]
///     #[orml_weight_meter::start_with(1_000_000)]
///     pub fn do_something(origin: OriginFor<T>, something: u32) ->
///	    DispatchResultWithPostInfo {
///         let who = ensure_signed(origin)?;
///         Self::inner_do_something(something);
///         // Emit an event.
///         Self::deposit_event(Event::SomethingStored(something, who));
///         Ok(PostDispatchInfo::from(Some(orml_weight_meter::used_weight())))
///     }
/// }
/// ```
pub use weight_meter_procedural::method_benchmarks;
