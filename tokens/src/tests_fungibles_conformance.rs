#![cfg(test)]

use super::*;
use crate::fungibles_conformance_tests;
use mock::*;
use paste::paste;

macro_rules! run_tests {
    ($path:path, $currency_id:expr, $($name:ident),*) => {
		$(
			paste! {
				#[test]
				fn [< $name _ $currency_id _dust_trap_on >]() {
					let trap_account = DustReceiverAccount::get();
					let builder = ExtBuilder::default();
					builder.build().execute_with(|| {
						<Tokens as fungibles::Mutate<_>>::set_balance($currency_id, &trap_account, Tokens::minimum_balance($currency_id));
						$path::$name::<
							Tokens,
							<Runtime as frame_system::Config>::AccountId,
						>($currency_id, Some(trap_account));
					});
				}

                #[test]
				fn [< $name _ $currency_id _dust_trap_off >]() {
					let trap_account = DustReceiverAccount::get();
					let builder = ExtBuilder::default();
					builder.build().execute_with(|| {
						GetDustReceiverAccount::set(None);
						$path::$name::<
							Tokens,
							<Runtime as frame_system::Config>::AccountId,
						>($currency_id, None);
					});
				}
			}
		)*
	};
	($path:path, $currency_id:expr) => {
		run_tests!(
			$path,
			$currency_id,
			mint_into_success,
			mint_into_overflow,
			mint_into_below_minimum,
			burn_from_exact_success,
			burn_from_best_effort_success,
			burn_from_exact_insufficient_funds,
			restore_success,
			restore_overflow,
			restore_below_minimum,
			shelve_success,
			shelve_insufficient_funds,
			transfer_success,
			transfer_expendable_all,
			transfer_expendable_dust,
			transfer_protect_preserve,
			set_balance_mint_success,
			set_balance_burn_success,
			can_deposit_success,
			can_deposit_below_minimum,
			can_deposit_overflow,
			can_withdraw_success,
			can_withdraw_reduced_to_zero,
			can_withdraw_balance_low,
			reducible_balance_expendable,
			reducible_balance_protect_preserve
		);
	};
}

run_tests!(fungibles_conformance_tests::inspect_mutate, DOT);
run_tests!(fungibles_conformance_tests::inspect_mutate, BTC);
run_tests!(fungibles_conformance_tests::inspect_mutate, ETH);
