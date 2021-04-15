# Xtokens Module

## Overview

The xtokens module provides cross-chain token transfer functionality, by cross-consensus
messages(XCM).

The xtokens module provides functions for
- Token transfer from parachains to relay chain.
- Token transfer between parachains, including relay chain tokens like DOT,
  KSM, and parachain tokens like ACA, aUSD.

## Notes

#### Integration tests

Integration tests could be done manually after integrating xtokens into runtime. To cover the full features, set up at least 4 relay chain validators and 3 collators of different parachains, and use dispatchable calls to include all these scenarios:

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
