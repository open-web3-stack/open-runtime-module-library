# Asset Registry Module

## Overview

This module provides functionality for storing asset metadata. For each asset, it stores the number of decimals, asset name, asset symbol, existential deposit and (optional) location. Additionally, it stores a value of a generic type that chains can use to store any other metadata that the parachain may need (such as the fee rate, for example). It is designed to be easy to integrate into xcm setups. Various default implementations are provided for this purpose.

The pallet only contains two extrinsics, `register_asset` and `update_asset`:

- `register_asset` creates a new asset
- `update_asset` modifies some (or all) of the fields of an existing asset
