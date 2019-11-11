#![cfg_attr(not(feature = "std"), no_std)]

use sr_primitives::traits::{MaybeSerializeDeserialize, Member, SimpleArithmetic};
use srml_support::{decl_event, decl_module, decl_storage, Parameter};
use srml_system::{self as system};
use traits::{DataProvider, PriceProvider};

pub trait Trait: system::Trait {
	type CurrencyId: Parameter + Member + SimpleArithmetic + Default + Copy + MaybeSerializeDeserialize;
	type Price: Parameter + Member + Copy + SimpleArithmetic + Ord;
	type Source: DataProvider<Self::CurrencyId, Self::Price>;
}

mod mock;
mod tests;

decl_storage! {
	trait Store for Module<T: Trait> as Prices { }
}

decl_event!(
	pub enum Event<T> where
		<T as system::Trait>::AccountId
	{
		Dummy(AccountId),
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin { }
}

impl<T: Trait> Module<T> {}

impl<T: Trait> PriceProvider<T::CurrencyId, T::Price> for Module<T> {
	fn get_price(base: T::CurrencyId, quote: T::CurrencyId) -> Option<T::Price> {
		let base_price_result: Option<T::Price> = T::Source::get(&base);
		let quote_price_result: Option<T::Price> = T::Source::get(&quote);

		if let (Some(base_price), Some(quote_price)) = (base_price_result, quote_price_result) {
			Some(quote_price / base_price)
		} else {
			None
		}
	}
}
