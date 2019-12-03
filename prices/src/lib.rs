#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{decl_module, decl_storage, Parameter};
use orml_traits::{DataProvider, PriceProvider};
use orml_utilities::FixedU128;
use sp_runtime::traits::{MaybeSerializeDeserialize, Member};

pub type Price = FixedU128;

pub trait Trait: frame_system::Trait {
	type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize;
	type Source: DataProvider<Self::CurrencyId, Price>;
}

mod mock;
mod tests;

decl_storage! {
	trait Store for Module<T: Trait> as Prices { }
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin { }
}

impl<T: Trait> Module<T> {}

impl<T: Trait> PriceProvider<T::CurrencyId, Price> for Module<T> {
	fn get_price(base_currency_id: T::CurrencyId, quote_currency_id: T::CurrencyId) -> Option<Price> {
		let base_price = T::Source::get(&base_currency_id)?;
		let quote_price = T::Source::get(&quote_currency_id)?;

		quote_price.checked_div(&base_price)
	}
}
