# Oracle module

### Overview

This module exposes capabilities for oracle operators to feed external offchain data.
The raw values can be combined to provide an aggregated value.

The data are submitted with unsigned transaction so it does not incure a transaction fee. However the data
still needs to be signed by a session key to prevent spam and ensure the integrity.
