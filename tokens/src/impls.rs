use codec::FullCodec;
use frame_support::traits::{
	fungible, fungibles,
	tokens::{DepositConsequence, WithdrawConsequence},
	Contains,
};
use sp_runtime::traits::AtLeast32BitUnsigned;
use sp_std::fmt::Debug;

pub struct Combiner<AccountId, TestKey, A, B>(sp_std::marker::PhantomData<(AccountId, TestKey, A, B)>);

impl<AccountId, TestKey, A, B> fungibles::Inspect<AccountId> for Combiner<AccountId, TestKey, A, B>
where
	TestKey: Contains<<B as fungibles::Inspect<AccountId>>::AssetId>,
	A: fungible::Inspect<AccountId, Balance = <B as fungibles::Inspect<AccountId>>::Balance>,
	B: fungibles::Inspect<AccountId>,
{
	type AssetId = <B as fungibles::Inspect<AccountId>>::AssetId;
	type Balance = <B as fungibles::Inspect<AccountId>>::Balance;

	fn total_issuance(asset: Self::AssetId) -> Self::Balance {
		if TestKey::contains(&asset) {
			A::total_issuance()
		} else {
			B::total_issuance(asset)
		}
	}

	fn minimum_balance(asset: Self::AssetId) -> Self::Balance {
		if TestKey::contains(&asset) {
			A::minimum_balance()
		} else {
			B::minimum_balance(asset)
		}
	}

	fn balance(asset: Self::AssetId, who: &AccountId) -> Self::Balance {
		if TestKey::contains(&asset) {
			A::balance(who)
		} else {
			B::balance(asset, who)
		}
	}

	fn reducible_balance(asset: Self::AssetId, who: &AccountId, keep_alive: bool) -> Self::Balance {
		if TestKey::contains(&asset) {
			A::reducible_balance(who, keep_alive)
		} else {
			B::reducible_balance(asset, who, keep_alive)
		}
	}

	fn can_deposit(asset: Self::AssetId, who: &AccountId, amount: Self::Balance) -> DepositConsequence {
		if TestKey::contains(&asset) {
			A::can_deposit(who, amount)
		} else {
			B::can_deposit(asset, who, amount)
		}
	}

	fn can_withdraw(
		asset: Self::AssetId,
		who: &AccountId,
		amount: Self::Balance,
	) -> WithdrawConsequence<Self::Balance> {
		if TestKey::contains(&asset) {
			A::can_withdraw(who, amount)
		} else {
			B::can_withdraw(asset, who, amount)
		}
	}
}

pub trait ConvertBalance<A, B> {
	fn convert_balance(amount: A) -> B;
	fn convert_balance_back(amount: B) -> A;
}

pub struct Mapper<AccountId, T, C, B>(sp_std::marker::PhantomData<(AccountId, T, C, B)>);
impl<AccountId, T, C, B> fungible::Inspect<AccountId> for Mapper<AccountId, T, C, B>
where
	T: fungible::Inspect<AccountId>,
	C: ConvertBalance<<T as fungible::Inspect<AccountId>>::Balance, B>,
	// TOOD: use trait Balance after https://github.com/paritytech/substrate/pull/9863 is available
	B: AtLeast32BitUnsigned + FullCodec + Copy + Default + Debug,
{
	type Balance = B;

	fn total_issuance() -> Self::Balance {
		C::convert_balance(T::total_issuance())
	}

	fn minimum_balance() -> Self::Balance {
		C::convert_balance(T::minimum_balance())
	}

	fn balance(who: &AccountId) -> Self::Balance {
		C::convert_balance(T::balance(who))
	}

	fn reducible_balance(who: &AccountId, keep_alive: bool) -> Self::Balance {
		C::convert_balance(T::reducible_balance(who, keep_alive))
	}

	fn can_deposit(who: &AccountId, amount: Self::Balance) -> DepositConsequence {
		T::can_deposit(who, C::convert_balance_back(amount))
	}

	fn can_withdraw(who: &AccountId, amount: Self::Balance) -> WithdrawConsequence<Self::Balance> {
		use WithdrawConsequence::*;
		let res = T::can_withdraw(who, C::convert_balance_back(amount));
		match res {
			WithdrawConsequence::ReducedToZero(b) => WithdrawConsequence::ReducedToZero(C::convert_balance(b)),
			NoFunds => NoFunds,
			WouldDie => WouldDie,
			UnknownAsset => UnknownAsset,
			Underflow => Underflow,
			Overflow => Overflow,
			Frozen => Frozen,
			Success => Success,
		}
	}
}
