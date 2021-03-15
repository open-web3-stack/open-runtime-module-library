# Weight Meter

## How to use WeightMeter
1. Include `WeightMeter` into your module Cargo.toml
```
orml-weight-meter = { version = "0.1.0", default-features = false }

std = [
    ...
    'orml-weight-meter/std',
]
runtime-benchmarks = [
	...
    'orml-weight-meter/runtime-benchmarks',
]

```

2. Add macro attribute on top of the module declaration
```
#[orml_weight_meter::method_benchmarks]
#[frame_support::pallet]
pub mod module {
	use super::*;
    ...
```

3. Add macro attribute on method you want to benchmark.
```
#[orml_weight_meter::weight(0)]
fn inner_do_something(something: u32) {
    // Update storage.
    Something::<T>::put(something);
}
```
Start with `0` and after the weights is generated then it can be replaced with generated weight. Macro will inject callable methods that wraps inner methods. Generated call will start with prefix `method_` followed by method name. This only works for methods with `orml_weight_meter::weight` attribute and only when running benchmarks.

4. Create benchmarks as we normally do. Just need to use prefix `method_` followed by
method name.
```
method_inner_do_something {
    let caller = whitelisted_caller();
}: _(frame_system::RawOrigin::Signed(caller), 10)
```
After running benchmarks we can replace `#[orml_weight_meter::weight(0)]` with 
 `#[orml_weight_meter::weight(T::WeightInfo::method_inner_do_something())]`.

5. Use WeightMeter on your calls by adding macro `#[orml_weight_meter::start_with(<base>)]` and at the end use `orml_weight_meter::used_weight()` to get used weight.
```
#[pallet::call]
impl<T: Config> Pallet<T> {
    #[pallet::weight(T::WeightInfo::do_something())]
    #[orml_weight_meter::start_with(1_000_000)]
    pub fn do_something(origin: OriginFor<T>, something: u32) -> DispatchResultWithPostInfo {
        let who = ensure_signed(origin)?;

        Self::inner_do_something(something);

        // Emit an event.
        Self::deposit_event(Event::SomethingStored(something, who));

        Ok(PostDispatchInfo::from(Some(orml_weight_meter::used_weight())))
    }
}
```
