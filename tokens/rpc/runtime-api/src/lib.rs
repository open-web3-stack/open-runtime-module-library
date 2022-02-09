//! Runtime API definition for transaction payment pallet.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use frame_support::pallet_prelude::*;
use sp_runtime::traits::{MaybeDisplay, MaybeSerializeDeserialize, Member};

sp_api::decl_runtime_apis! {
	pub trait TokensApi<CurrencyId, Balance> where
		Balance: Codec + MaybeDisplay,
		CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord
	{
		fn query_existential_deposit(currency_id: CurrencyId) -> Balance;
	}
}
