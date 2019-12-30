# open-runtime-module-library
The Open Runtime Module Library (ORML) is a community maintained collection of Substrate runtime modules.

## Runtime modules

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

## Makefile targets

- `make check`
	- Type check the code, without std feature, exclduing tests.
- `make check-tests`
	- Type check the code, with std feature, including tests.
- `make test`
	- Run tests.
