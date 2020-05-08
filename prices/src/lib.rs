#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::Parameter;
use orml_traits::{DataProvider, PriceProvider};
use orml_utilities::FixedU128;
use sp_runtime::traits::{MaybeSerializeDeserialize, Member};
use sp_std::marker::PhantomData;

pub type Price = FixedU128;

mod tests;

pub struct DefaultPriceProvider<CurrencyId, Source>(PhantomData<(CurrencyId, Source)>);

impl<CurrencyId, Source> PriceProvider<CurrencyId, Price> for DefaultPriceProvider<CurrencyId, Source>
where
	CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize,
	Source: DataProvider<CurrencyId, Price>,
{
	fn get_price(base_currency_id: CurrencyId, quote_currency_id: CurrencyId) -> Option<Price> {
		let base_price = Source::get(&base_currency_id)?;
		let quote_price = Source::get(&quote_currency_id)?;

		base_price.checked_div(&quote_price)
	}
}
