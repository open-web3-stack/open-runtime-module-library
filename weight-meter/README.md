# Weight Meter

Include `WeightMeter` into your module Cargo.toml
```
[dependencies]
orml-weight-meter = { version = "..", default-features = false }

std = [
    ..
    'orml-weight-meter/std',
]
runtime-benchmarks = [
    ..
    'orml-weight-meter/runtime-benchmarks',
]

```