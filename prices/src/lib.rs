#![cfg_attr(not(feature = "std"), no_std)]

use rstd::result;
use sr_primitives::traits::{MaybeSerializeDeserialize, Member, SimpleArithmetic};
use srml_support::{decl_error, decl_event, decl_module, decl_storage, ensure, Parameter};
use srml_system::{self as system, ensure_signed};
use traits::{DataProvider, PriceProvider};

pub trait Trait: system::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type CurrencyId: Parameter + Member + SimpleArithmetic + Default + Copy + MaybeSerializeDeserialize;
	type Price;
	type Source: PriceProvider<Self::CurrencyId, Self::Price>;
}
