pub use crate::*;

use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_runtime::traits::SaturatedConversion;

/// Helper trait for benchmarking.
pub trait BenchmarkHelper<CurrencyId, Balance> {
	/// Returns a currency id and amount to be used in benchmarking.
	fn get_currency_id_and_amount() -> Option<(CurrencyId, Balance)>;
}

impl<CurrencyId, Balance> BenchmarkHelper<CurrencyId, Balance> for () {
	fn get_currency_id_and_amount() -> Option<(CurrencyId, Balance)> {
		None
	}
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn transfer() {
		let from: T::AccountId = account("from", 0, 0);

		let (currency_id, amount) = T::BenchmarkHelper::get_currency_id_and_amount().unwrap();

		assert_ok!(<Pallet::<T> as MultiCurrencyExtended<_>>::update_balance(
			currency_id,
			&from,
			amount.saturated_into()
		));

		let to: T::AccountId = account("to", 0, 0);
		let to_lookup = <T as frame_system::Config>::Lookup::unlookup(to.clone());

		#[extrinsic_call]
		_(RawOrigin::Signed(from), to_lookup, currency_id, amount);

		assert_eq!(Pallet::<T>::total_balance(currency_id, &to), amount);
	}

	#[benchmark]
	fn transfer_all() {
		let from: T::AccountId = account("from", 0, 0);

		let (currency_id, amount) = T::BenchmarkHelper::get_currency_id_and_amount().unwrap();

		assert_ok!(<Pallet::<T> as MultiCurrencyExtended<_>>::update_balance(
			currency_id,
			&from,
			amount.saturated_into()
		));

		let to: T::AccountId = account("to", 0, 0);
		let to_lookup = <T as frame_system::Config>::Lookup::unlookup(to.clone());

		#[extrinsic_call]
		_(RawOrigin::Signed(from.clone()), to_lookup, currency_id, false);

		assert_eq!(
			<Pallet::<T> as MultiCurrency<_>>::total_balance(currency_id, &from),
			0u32.into()
		);
	}

	#[benchmark]
	fn transfer_keep_alive() {
		let from: T::AccountId = account("from", 0, 0);

		let (currency_id, amount) = T::BenchmarkHelper::get_currency_id_and_amount().unwrap();

		assert_ok!(<Pallet::<T> as MultiCurrencyExtended<_>>::update_balance(
			currency_id,
			&from,
			amount.saturating_mul(2u32.into()).saturated_into()
		));

		let to: T::AccountId = account("to", 0, 0);
		let to_lookup = <T as frame_system::Config>::Lookup::unlookup(to.clone());

		#[extrinsic_call]
		_(RawOrigin::Signed(from), to_lookup, currency_id, amount);

		assert_eq!(
			<Pallet::<T> as MultiCurrency<_>>::total_balance(currency_id, &to),
			amount
		);
	}

	#[benchmark]
	fn force_transfer() {
		let from: T::AccountId = account("from", 0, 0);
		let from_lookup = <T as frame_system::Config>::Lookup::unlookup(from.clone());

		let (currency_id, amount) = T::BenchmarkHelper::get_currency_id_and_amount().unwrap();

		assert_ok!(<Pallet::<T> as MultiCurrencyExtended<_>>::update_balance(
			currency_id,
			&from,
			amount.saturated_into()
		));

		let to: T::AccountId = account("to", 0, 0);
		let to_lookup = <T as frame_system::Config>::Lookup::unlookup(to.clone());

		#[extrinsic_call]
		_(RawOrigin::Root, from_lookup, to_lookup, currency_id, amount);

		assert_eq!(
			<Pallet::<T> as MultiCurrency<_>>::total_balance(currency_id, &to),
			amount
		);
	}

	#[benchmark]
	fn set_balance() {
		let who: T::AccountId = account("who", 0, 0);
		let who_lookup = <T as frame_system::Config>::Lookup::unlookup(who.clone());

		let (currency_id, amount) = T::BenchmarkHelper::get_currency_id_and_amount().unwrap();

		#[extrinsic_call]
		_(RawOrigin::Root, who_lookup, currency_id, amount, amount);

		assert_eq!(
			<Pallet::<T> as MultiCurrency<_>>::total_balance(currency_id, &who),
			amount.saturating_mul(2u32.into())
		);
	}

	impl_benchmark_test_suite! {
		Pallet,
		crate::mock::ExtBuilder::default().build(),
		crate::mock::Runtime,
	}
}
