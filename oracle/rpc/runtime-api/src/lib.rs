//! Runtime API definition for oracle module.

#![cfg_attr(not(feature = "std"), no_std)]

// The `too_many_arguments` warning originates from `decl_runtime_apis` macro.
#![allow(clippy::too_many_arguments)]

use codec::Codec;
use sp_std::prelude::Vec;

sp_api::decl_runtime_apis! {
	pub trait OracleApi<Key, Value> where
		Key: Codec,
		Value: Codec,
	{
		fn get_value(key: Key) -> Option<Value>;
		fn get_all_values() -> Vec<(Key, Option<Value>)>;
	}
}
