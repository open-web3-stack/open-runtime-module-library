# open-runtime-module-library
The Open Runtime Module Library (ORML) is a community maintained collection of Substrate runtime modules.

## Runtime Modules Overview

- orml-traits
    - Shared traits including `BasicCurrency`, `MultiCurrency`, `Auction` and more.
- orml-utilities
	- Various utilities including `FixedU128` and `LinkedList`.
- orml-tokens
    - Fungible tokens module that implements `MultiCurrency` trait.
- orml-currencies
	- Provide `MultiCurrency` implementation using `pallet-balances` and `orml-tokens` module.
- orml-oracle
    - Oracle module that makes off-chain data available on-chain.
- orml-prices
	- Provide basic asset price abstraction.
- orml-auction
	- Auction module that implements `Auction` trait.
- orml-vesting
    - Provides scheduled balance locking mechanism, in a *graded vesting* way.

## Installation

### Runtime `Cargo.toml`

To add an `ORML` module to your runtime, simply include the following to your runtime's `Cargo.toml` file. For instance, to add `orml-tokens` module:

```TOML
[dependencies]
# --snip--
orml-tokens = { git = "https://github.com/laminar-protocol/open-runtime-module-library.git", default-features = false }
```

and update your runtime's `std` feature to include this module:

```TOML
std = [
    # --snip--
    'orml-tokens/std',
]
```

### Runtime `lib.rs`

You should implement it's trait like so:

```rust
/// Used for orml_tokens
impl orml_tokens::Trait for Runtime {
	type Event = Event;
	// --snip--
}
```

and include it in your `construct_runtime!` macro:

```rust
Tokens: orml_tokens::{Module, Storage, Call, Event<T>, Config<T>},
```

## Development

### Makefile targets

- `make check`
	- Type check the code, without std feature, exclduing tests.
- `make check-tests`
	- Type check the code, with std feature, including tests.
- `make test`
	- Run tests.
