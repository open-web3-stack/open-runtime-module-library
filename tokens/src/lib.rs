#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use rstd::{fmt::Debug, result};
use srml_support::{decl_event, decl_module, decl_storage, ensure, Parameter};
use sr_primitives::traits::{CheckedAdd, CheckedSub, MaybeSerializeDeserialize, Member, SimpleArithmetic};
// FIXME: `srml-` prefix should be used for all srml modules, but currently `srml_system`
// would cause compiling error in `decl_module!` and `construct_runtime!`
// #3296 https://github.com/paritytech/substrate/issues/3295
use srml_system as system;

use traits::MultiCurrency;

pub trait Trait: srml_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as srml_system::Trait>::Event>;
	type Balance: Parameter + Member + SimpleArithmetic + Codec + Default + Copy + MaybeSerializeDeserialize + Debug;
	type CurrencyId: Parameter + Member + SimpleArithmetic + Default + Copy + MaybeSerializeDeserialize;
}

decl_storage! {
	trait Store for Module<T: Trait> as Tokens {
		/// The total issuance of a token type;
		pub TotalIssuance get(fn total_issuance) build(|config: &GenesisConfig<T>| {
			let issuance = config.initial_balance * (config.endowed_accounts.len() as u32).into();
			config.tokens.iter().map(|id| (id.clone(), issuance)).collect::<Vec<_>>()
		}): map T::CurrencyId => T::Balance;

		/// The balance of a token type under an account.
		pub Balance get(fn balance): double_map T::CurrencyId, twox_128(T::AccountId) => T::Balance;
	}
	add_extra_genesis {
		config(tokens): Vec<T::CurrencyId>;
		config(initial_balance): T::Balance;
		config(endowed_accounts): Vec<T::AccountId>;
	}
}

decl_event!(
	pub enum Event<T> where
		<T as srml_system::Trait>::AccountId
	{
		Dummy(AccountId),
	}
);

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

	}
}

impl<T: Trait> Module<T> {}

impl<T: Trait> MultiCurrency<T::AccountId> for Module<T> {
	type Balance = T::Balance;
	type CurrencyId = T::CurrencyId;

	fn total_inssuance(currency_id: &Self::CurrencyId) -> Self::Balance {
		<TotalIssuance<T>>::get(currency_id)
	}

	fn balance(currency_id: &Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
		<Balance<T>>::get(currency_id, who)
	}

	fn transfer(
		currency_id: &Self::CurrencyId,
		from: &T::AccountId,
		to: &T::AccountId,
		amount: Self::Balance,
	) -> result::Result<(), &'static str> {
		ensure!(Self::balance(currency_id, from) >= amount, "balance too low to send amount");

		if from != to {
			<Balance<T>>::mutate(currency_id, from, |balance| *balance -= amount);
			<Balance<T>>::mutate(currency_id, to, |balance| *balance += amount);
		}

		Ok(())
	}

	fn mint(
		currency_id: &Self::CurrencyId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> result::Result<(), &'static str> {
		ensure!(
			Self::total_inssuance(currency_id).checked_add(&amount).is_some(),
			"total issuance overflow",
		);

		<TotalIssuance<T>>::mutate(currency_id, |v| *v += amount);
		<Balance<T>>::mutate(currency_id, who, |v| *v += amount);

		Ok(())
	}

	fn burn(
		currency_id: &Self::CurrencyId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> result::Result<(), &'static str> {
		ensure!(
			Self::balance(currency_id, who).checked_sub(&amount).is_some(),
			"insufficient balance to burn",
		);

		<TotalIssuance<T>>::mutate(currency_id, |v| *v -= amount);
		<Balance<T>>::mutate(currency_id, who, |v| *v -= amount);

		Ok(())
	}

	fn slash(
		currency_id: &Self::CurrencyId,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> Self::Balance {
		let actual_amount = Self::balance(currency_id, who).max(amount);
		<Balance<T>>::mutate(currency_id, who, |v| *v -= actual_amount);
		actual_amount
	}
}
