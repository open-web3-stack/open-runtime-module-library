//! Runtime API definition for transaction payment pallet.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use sp_runtime::traits::MaybeDisplay;
use sp_std::collections::btree_map::BTreeMap;
use sp_std::prelude::*;

sp_api::decl_runtime_apis! {
	pub trait TokensApi<CurrencyId, Balance> where
		Balance: Codec + MaybeDisplay,
		CurrencyId: Codec + Ord
	{
		fn query_existential_deposit(currency_id: CurrencyId) -> Balance;
		fn existential_deposits() -> BTreeMap<CurrencyId, Balance>;
	}
}
