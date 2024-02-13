# Xtokens Module

## Overview

The `xtokens` module provides cross-chain token transfer functionality, by cross-consensus messages(XCM).

The `xtokens` module provides functions for
- Token transfer from parachains to relay chain.
- Token transfer between parachains, including relay chain tokens like DOT,
  KSM, and parachain tokens like ACA, aUSD, USDT.

## Notes

#### Integration tests

Integration tests could be done manually after integrating `xtokens` into a runtime. To cover the full features, set up at least 4 relay chain validators and 3 collators of different parachains, and use dispatchable calls to include all these scenarios:

- Transfer relay chain tokens to relay chain.
- Transfer tokens issued by parachain A, from parachain A to parachain B.
  - Sending the tx from parachain A.
  - Set the destination as Parachain B.
  - Set the currency ID as parachain A token.
- Transfer tokens issued by parachain B, from parachain A to parachain B.
  - Sending the tx from parachain A.
  - Set the destination as Parachain B.
  - Set the currency ID as parachain B token.
- Transfer tokens issued by parachain C, from parachain A to parachain B.
  - Sending the tx from parachain A.
  - Set the destination as Parachain B.
  - Set the currency ID as parachain C token.


#### Transfer multiple currencies

- Transfer relay chain tokens to relay chain, and use relay chain token as fee
- Transfer relay chain tokens to parachain, and use relay chain token as fee
- Transfer tokens issued by parachain A, from parachain A to parachain B, and use parachain A token as a fee
- Transfer tokens issued by parachain B, from parachain A to parachain B, and use parachain B token as a fee
- Transfer tokens issued by parachain C, from parachain A to parachain B, and use parachain C token as a fee
- Transfer tokens issued by parachain B, from parachain A to parachain B, and use relay chain token as fee

Notice, in the case of parachain A transfer parachain B token to parachain B, and use relay chain token as fee. Because fee asset is relaychain token, and non fee asset is parachain B token, this is two different chain. We call chain of fee asset as `fee_reserve`, and chain of non fee asset as `non_fee_reserve`. And in this case `fee_reserve` location is also refer to destination parachain.

The current implementation sends two XCM from a sender parachain. First XCM is sent to the fee reserve chain which will also route the XCM message to the destination parachain. Second XCM directly sent to destination parachain.

The fee amount in fee asset is split into two parts:
1. Fee asset sent to fee reserve chain = fee_amount - min_xcm_fee
2. Fee asset sent to dest reserve chain = min_xcm_fee

Parachains should implement config `MinXcmFee` in `xtokens` module config:

```rust
parameter_type_with_key! {
	pub MinXcmReserveFee: |location: Location| -> Option<u128> {
		#[allow(clippy::match_ref_pats)] // false positive
		match (location.parents, location.first_interior()) {
			(1, Some(Parachain(parachains::statemine::ID))) => Some(4_000_000_000),
			_ => None,
		}
	};
}
```

If Parachain don't want have this case, can simply return None. A default implementation is provided by `DisabledParachainFee` in `xcm-support`.

```rust
parameter_type_with_key! {
	pub ParachainMinFee: |_location: Location| -> Option<u128> {
		None
	};
}
```

Notice the implementation for now also relies on `SelfLocation` which is already in `xtokens` config. The `SelfLocation` can be set to the absolute view `(1, Parachain(THIS_PARACHAIN_ID))` and refers to the sender parachain. The alternative is to set `SelfLocation` to relative view `(0, Here)` to adhere to Polkadot guidelines.

We use `SelfLocation` to fund fee to sender's parachain sovereign account on destination parachain, which asset is originated from sender account on sender parachain. This means if user setup too much fee, the fee will not returned to user, instead deposit to sibling parachain sovereign account on destination parachain.
