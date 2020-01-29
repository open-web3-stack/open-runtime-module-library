//! Runtime API definition for oracle module.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;

sp_api::decl_runtime_apis! {
	pub trait OracleApi<Key, Value> where
		Key: Codec,
		Value: Codec,
	{
		fn get_no_op(key: Key) -> Option<Value>;
	}
}
