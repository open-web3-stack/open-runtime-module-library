#![cfg(test)]

use super::*;
use cumulus_primitives_core::ParaId;
use frame_support::{assert_ok, traits::Currency};
use mock::*;
use orml_traits::MultiCurrency;
use polkadot_parachain::primitives::AccountIdConversion;
use sp_runtime::AccountId32;
use xcm::v0::{Junction, NetworkId};
use xcm_simulator::TestExt;

fn para_a_account() -> AccountId32 {
	ParaId::from(1).into_account()
}

#[test]
fn send_relay_chain_asset_to_relay_chain() {
	TestNetwork::reset();

	MockRelay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 100);
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaAXtokens::transfer(
			Some(ALICE).into(),
			CurrencyId::R,
			30,
			(
				Junction::Parent,
				Junction::AccountId32 {
					network: NetworkId::Polkadot,
					id: BOB.into(),
				},
			)
				.into(),
		));
		assert_eq!(ParaATokens::free_balance(CurrencyId::R, &ALICE), 70);
	});

	MockRelay::execute_with(|| {
		assert_eq!(RelayBalances::free_balance(&para_a_account()), 70);
		assert_eq!(RelayBalances::free_balance(&BOB), 30);
	});
}
