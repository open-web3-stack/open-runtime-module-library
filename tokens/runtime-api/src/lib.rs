//! Runtime API definition for orml tokens pallet.

#![cfg_attr(not(feature = "std"), no_std)]
// The `too_many_arguments` warning originates from `decl_runtime_apis` macro.
#![allow(clippy::too_many_arguments)]
// The `unnecessary_mut_passed` warning originates from `decl_runtime_apis` macro.
#![allow(clippy::unnecessary_mut_passed)]

use parity_scale_codec::Codec;

sp_api::decl_runtime_apis! {
	pub trait TokensApi<CurrencyId, Balance> where
		Balance: Codec,
		CurrencyId: Codec
	{
		fn query_existential_deposit(currency_id: CurrencyId) -> Balance;
	}
}
