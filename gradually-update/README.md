# Gradually-update module

### Overview

Gradually-update module provides a way to adjust numeric parameter such as stability fee or liquidation gradually. The update code should be able to handle different numeric types such as `u32`, `u128`, `Permill`, `FixedU128`. All the values are assumed to be little-endian and unsigned.
