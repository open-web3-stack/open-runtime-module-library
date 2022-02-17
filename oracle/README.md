# Oracle module

### Overview

This module exposes capabilities for oracle operators to feed external offchain data.
The raw values can be combined to provide an aggregated value.

The data is valid only if feeded by an authorized operator. This module implements `frame_support::traits::InitializeMembers` and `frame_support::traits::ChangeMembers`, to provide a way to manage operators membership. Typically it could be leveraged to `pallet_membership` in FRAME.
