# open-runtime-module-library
The Open Runtime Module Library (ORML) is a community maintained collection of Substrate runtime modules.

## Runtime modules

- orml-traits
    - Shared traits including `MultiCurrency`.
- orml-tokens
    - Fungible tokens module that implements `MultiCurrency` trait.
- orml-oracle
    - Oracle module that makes off-chain data available on-chain.

## Makefile targets

- `make check`
	- Type check the code, without std feature, exclduing tests.
- `make check-tests`
	- Type check the code, with std feature, including tests.
- `make test`
	- Run tests.
