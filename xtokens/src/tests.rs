#![cfg(test)]

use super::*;
use frame_support::{assert_err, assert_noop, assert_ok, traits::Currency};
use mock::*;
use orml_traits::{ConcreteFungibleAsset, MultiCurrency};
use xcm_simulator::TestExt;

// Not used in any unit tests, but it's super helpful for debugging. Let's
// keep it here.
#[allow(dead_code)]
fn print_events<Runtime: frame_system::Config>(name: &'static str) {
	println!("------ {:?} events -------", name);
	frame_system::Pallet::<Runtime>::events()
		.iter()
		.for_each(|r| println!("> {:?}", r.event));
}

#[test]
fn send_relay_chain_asset_to_relay_chain() {
	TestNet::reset();

	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 1_000);
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer(
			Some(ALICE).into(),
			CurrencyId::R,
			500,
			Box::new(
				MultiLocation::new(
					1,
					X1(Junction::AccountId32 {
						network: NetworkId::Any,
						id: BOB.into(),
					})
				)
				.into()
			),
			40,
		));
		assert_eq!(ParaTokens::free_balance(CurrencyId::R, &ALICE), 500);
	});

	Relay::execute_with(|| {
		assert_eq!(RelayBalances::free_balance(&para_a_account()), 500);
		assert_eq!(RelayBalances::free_balance(&BOB), 460);
	});
}

#[test]
fn cannot_lost_fund_on_send_failed() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::A, &ALICE, 1_000));
		assert_noop!(
			ParaXTokens::transfer(
				Some(ALICE).into(),
				CurrencyId::A,
				500,
				Box::new(
					(
						Parent,
						Parachain(100),
						Junction::AccountId32 {
							network: NetworkId::Kusama,
							id: BOB.into(),
						},
					)
						.into()
				),
				40,
			),
			Error::<para::Runtime>::XcmExecutionFailed
		);

		assert_eq!(ParaTokens::free_balance(CurrencyId::R, &ALICE), 1_000);
	});
}

#[test]
fn send_relay_chain_asset_to_sibling() {
	TestNet::reset();

	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 1000);
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer(
			Some(ALICE).into(),
			CurrencyId::R,
			500,
			Box::new(
				MultiLocation::new(
					1,
					X2(
						Parachain(2),
						Junction::AccountId32 {
							network: NetworkId::Any,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			40,
		));
		assert_eq!(ParaTokens::free_balance(CurrencyId::R, &ALICE), 500);
	});

	Relay::execute_with(|| {
		assert_eq!(RelayBalances::free_balance(&para_a_account()), 500);
		assert_eq!(RelayBalances::free_balance(&para_b_account()), 460);
	});

	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::R, &BOB), 420);
	});
}

#[test]
fn send_sibling_asset_to_reserve_sibling() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &ALICE, 1_000));
	});

	ParaB::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &sibling_a_account(), 1_000));
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer(
			Some(ALICE).into(),
			CurrencyId::B,
			500,
			Box::new(
				(
					Parent,
					Parachain(2),
					Junction::AccountId32 {
						network: NetworkId::Any,
						id: BOB.into(),
					},
				)
					.into()
			),
			40,
		));

		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &ALICE), 500);
	});

	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &sibling_a_account()), 500);
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &BOB), 460);
	});
}

#[test]
fn send_sibling_asset_to_non_reserve_sibling() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &ALICE, 1_000));
	});

	ParaB::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::B, &sibling_a_account(), 1_000));
	});

	ParaA::execute_with(|| {
		assert_ok!(ParaXTokens::transfer(
			Some(ALICE).into(),
			CurrencyId::B,
			500,
			Box::new(
				MultiLocation::new(
					1,
					X2(
						Parachain(3),
						Junction::AccountId32 {
							network: NetworkId::Any,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			40
		));
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &ALICE), 500);
	});

	// check reserve accounts
	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &sibling_a_account()), 500);
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &sibling_c_account()), 460);
	});

	ParaC::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::B, &BOB), 420);
	});
}

#[test]
fn send_self_parachain_asset_to_sibling() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::A, &ALICE, 1_000));

		assert_ok!(ParaXTokens::transfer(
			Some(ALICE).into(),
			CurrencyId::A,
			500,
			Box::new(
				MultiLocation::new(
					1,
					X2(
						Parachain(2),
						Junction::AccountId32 {
							network: NetworkId::Any,
							id: BOB.into(),
						}
					)
				)
				.into()
			),
			40,
		));

		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &ALICE), 500);
		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &sibling_b_account()), 500);
	});

	ParaB::execute_with(|| {
		assert_eq!(ParaTokens::free_balance(CurrencyId::A, &BOB), 460);
	});
}

#[test]
fn transfer_no_reserve_assets_fails() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_noop!(
			ParaXTokens::transfer_multiasset(
				Some(ALICE).into(),
				Box::new((X1(GeneralKey("B".into())).into(), 100).into()),
				Box::new(
					(
						Parent,
						Parachain(2),
						Junction::AccountId32 {
							network: NetworkId::Any,
							id: BOB.into()
						}
					)
						.into()
				),
				50,
			),
			Error::<para::Runtime>::AssetHasNoReserve
		);
	});
}

#[test]
fn transfer_to_self_chain_fails() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_noop!(
			ParaXTokens::transfer_multiasset(
				Some(ALICE).into(),
				Box::new(MultiAsset::sibling_parachain_asset(1, "A".into(), 100).into()),
				Box::new(
					MultiLocation::new(
						1,
						X2(
							Parachain(1),
							Junction::AccountId32 {
								network: NetworkId::Any,
								id: BOB.into()
							}
						)
					)
					.into()
				),
				50,
			),
			Error::<para::Runtime>::NotCrossChainTransfer
		);
	});
}

#[test]
fn transfer_to_invalid_dest_fails() {
	TestNet::reset();

	ParaA::execute_with(|| {
		assert_noop!(
			ParaXTokens::transfer_multiasset(
				Some(ALICE).into(),
				Box::new(MultiAsset::sibling_parachain_asset(1, "A".into(), 100).into()),
				Box::new(
					MultiLocation::new(
						0,
						X1(Junction::AccountId32 {
							network: NetworkId::Any,
							id: BOB.into()
						})
					)
					.into()
				),
				50,
			),
			Error::<para::Runtime>::InvalidDest
		);
	});
}

#[test]
fn send_as_sovereign() {
	TestNet::reset();

	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 1_000_000_000_000);
	});

	ParaA::execute_with(|| {
		use xcm::latest::OriginKind::SovereignAccount;

		let call =
			relay::Call::System(frame_system::Call::<relay::Runtime>::remark_with_event { remark: vec![1, 1, 1] });
		let assets: MultiAsset = (Here, 1_000_000_000_000).into();
		assert_ok!(para::OrmlXcm::send_as_sovereign(
			para::Origin::root(),
			Box::new(MultiLocation::parent()),
			Box::new(Xcm(vec![
				WithdrawAsset(assets.clone().into()),
				BuyExecution {
					fees: assets,
					weight_limit: Limited(2_000_000_000)
				},
				Instruction::Transact {
					origin_type: SovereignAccount,
					require_weight_at_most: 1_000_000_000,
					call: call.encode().into(),
				}
			]))
		));
	});

	Relay::execute_with(|| {
		assert!(relay::System::events().iter().any(|r| {
			matches!(
				r.event,
				relay::Event::System(frame_system::Event::<relay::Runtime>::Remarked(_, _))
			)
		}));
	})
}

#[test]
fn send_as_sovereign_fails_if_bad_origin() {
	TestNet::reset();

	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 1_000_000_000_000);
	});

	ParaA::execute_with(|| {
		use xcm::latest::OriginKind::SovereignAccount;

		let call =
			relay::Call::System(frame_system::Call::<relay::Runtime>::remark_with_event { remark: vec![1, 1, 1] });
		let assets: MultiAsset = (Here, 1_000_000_000_000).into();
		assert_err!(
			para::OrmlXcm::send_as_sovereign(
				para::Origin::signed(ALICE),
				Box::new(MultiLocation::parent()),
				Box::new(Xcm(vec![
					WithdrawAsset(assets.clone().into()),
					BuyExecution {
						fees: assets,
						weight_limit: Limited(10_000_000)
					},
					Instruction::Transact {
						origin_type: SovereignAccount,
						require_weight_at_most: 1_000_000_000,
						call: call.encode().into(),
					}
				]))
			),
			DispatchError::BadOrigin,
		);
	});
}

#[test]
fn para_transact_to_relay_remark_use_sovereign_account() {
	Relay::execute_with(|| {
		let _ = RelayBalances::deposit_creating(&para_a_account(), 6030);
	});

	ParaA::execute_with(|| {
		parachain_transact_to_relaychian_remark();
	});

	Relay::execute_with(|| {
		use relay::{Event, System};
		assert!(System::events()
			.iter()
			.any(|r| matches!(r.event, Event::System(frame_system::Event::Remarked(_, _)))));
	});
}

#[test]
fn relay_transact_to_para_remark_use_default_sovereign_account() {
	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::R, &DEFAULT, 6030));
	});

	relaychain_transact_to_parachain_remark(Here, 6030);

	ParaA::execute_with(|| {
		use para::{Event, System};
		assert!(System::events()
			.iter()
			.any(|r| matches!(r.event, Event::System(frame_system::Event::Remarked(_, _)))));
	});
}

#[test]
fn relay_transact_to_para_remark_use_normal_account() {
	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::R, &ALICE, 6040));
		assert_eq!(7040, ParaTokens::free_balance(CurrencyId::R, &ALICE));
	});

	let alice = Junctions::X1(Junction::AccountId32 {
		network: NetworkId::Kusama,
		id: ALICE.into(),
	});
	relaychain_transact_to_parachain_remark(alice.clone(), 6040);

	ParaA::execute_with(|| {
		use para::{Event, System};
		assert_eq!(1000, ParaTokens::free_balance(CurrencyId::R, &ALICE));
		assert!(System::events()
			.iter()
			.any(|r| matches!(r.event, Event::System(frame_system::Event::Remarked(_, _)))));
		System::reset_events();
	});
	relaychain_transact_to_parachain_remark(alice.clone(), 100);

	ParaA::execute_with(|| {
		use para::{Event, System};
		assert_eq!(900, ParaTokens::free_balance(CurrencyId::R, &ALICE));
		assert_eq!(
			System::events()
				.iter()
				.find(|r| matches!(r.event, Event::System(frame_system::Event::Remarked(_, _)))),
			None
		);
	});
}

#[test]
fn relay_transact_to_para_transfer_use_normal_account() {
	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::R, &ALICE, 195952040));
		assert_eq!(195953040, ParaTokens::free_balance(CurrencyId::R, &ALICE));
		let _ = ParaBalances::deposit_creating(&ALICE, 1_000);
		assert_eq!(1000, ParaBalances::free_balance(&ALICE));
	});

	let alice = Junctions::X1(Junction::AccountId32 {
		network: NetworkId::Kusama,
		id: ALICE.into(),
	});
	relaychain_transact_to_parachain_transfer(alice.clone(), 195952040, 500);

	ParaA::execute_with(|| {
		use para::{Event, System};
		assert_eq!(1000, ParaTokens::free_balance(CurrencyId::R, &ALICE));
		assert_eq!(500, ParaBalances::free_balance(&ALICE));
		assert_eq!(500, ParaBalances::free_balance(&BOB));
		assert!(System::events()
			.iter()
			.any(|r| matches!(r.event, Event::Balances(pallet_balances::Event::Transfer(_, _, _)))));
		System::reset_events();
	});

	relaychain_transact_to_parachain_transfer(alice.clone(), 100, 100);

	ParaA::execute_with(|| {
		use para::{Event, System};
		assert_eq!(900, ParaTokens::free_balance(CurrencyId::R, &ALICE));
		assert_eq!(500, ParaBalances::free_balance(&ALICE));
		assert_eq!(500, ParaBalances::free_balance(&BOB));
		assert_eq!(
			System::events()
				.iter()
				.find(|r| matches!(r.event, Event::Balances(pallet_balances::Event::Transfer(_, _, _)))),
			None
		);
	});
}

#[test]
fn para_transact_to_sibling_remark_use_sovereign_account() {
	ParaB::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::R, &sibling_a_account(), 6030));
	});

	parachain_transact_to_sibling_remark(Here, 6030);

	ParaB::execute_with(|| {
		use para::{Event, System};
		assert_eq!(0, ParaTokens::free_balance(CurrencyId::R, &sibling_a_account()));
		assert!(System::events()
			.iter()
			.any(|r| matches!(r.event, Event::System(frame_system::Event::Remarked(_, _)))));
	});
}

#[test]
fn para_transact_to_sibling_remark_use_account_failed() {
	let alice = Junctions::X1(Junction::AccountId32 {
		network: NetworkId::Any,
		id: ALICE.into(),
	});

	// the origin of `WithdrawAsset` in the context of destination parachain is
	// `(Parent, Parachain(1), Alice)` and it get error when convert by
	// `LocationToAccountId`.
	parachain_transact_to_sibling_remark(alice, 6040);

	ParaB::execute_with(|| {
		use para::{Event, System};
		assert_eq!(
			System::events()
				.iter()
				.find(|r| matches!(r.event, Event::System(frame_system::Event::Remarked(_, _)))),
			None
		);
	});
}

#[test]
fn relay_transact_to_para_unsupport_kind_failed() {
	ParaA::execute_with(|| {
		assert_ok!(ParaTokens::deposit(CurrencyId::R, &DEFAULT, 6040));
	});

	use para::{Call, Runtime};
	let call = Call::System(frame_system::Call::<Runtime>::remark_with_event { remark: vec![1, 2, 3] });
	let assets: MultiAsset = (Parent, 6040).into();
	let alice = Junctions::X1(Junction::AccountId32 {
		network: NetworkId::Any,
		id: ALICE.into(),
	});

	Relay::execute_with(|| {
		let xcm = vec![
			WithdrawAsset(assets.clone().into()),
			BuyExecution {
				fees: assets,
				weight_limit: Limited(6040),
			},
			Transact {
				origin_type: OriginKind::Native,
				require_weight_at_most: 6000 as u64,
				call: call.encode().into(),
			},
		];
		assert_ok!(RelayChainPalletXcm::send_xcm(alice, Parachain(1).into(), Xcm(xcm),));
	});

	ParaA::execute_with(|| {
		use para::{Event, System};
		assert_eq!(
			System::events()
				.iter()
				.find(|r| matches!(r.event, Event::System(frame_system::Event::Remarked(_, _)))),
			None
		);
	});
}

fn relaychain_transact_to_parachain_remark(junctions: Junctions, amount: u128) {
	use para::{Call, Runtime};
	let call = Call::System(frame_system::Call::<Runtime>::remark_with_event { remark: vec![1, 2, 3] });
	let assets: MultiAsset = (Parent, amount).into();

	let limit: u64 = match junctions {
		Here => 6030,
		_ => 6040,
	};

	Relay::execute_with(|| {
		let xcm = vec![
			WithdrawAsset(assets.clone().into()),
			BuyExecution {
				fees: assets,
				weight_limit: Limited(limit),
			},
			Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: 6000 as u64,
				call: call.encode().into(),
			},
		];
		assert_ok!(RelayChainPalletXcm::send_xcm(junctions, Parachain(1).into(), Xcm(xcm),));
	});
}

fn relaychain_transact_to_parachain_transfer(junctions: Junctions, amount: u128, transfer_amount: u128) {
	use para::{Call, Runtime};
	let call = Call::Balances(pallet_balances::Call::<Runtime>::transfer {
		dest: BOB,
		value: transfer_amount,
	});
	let assets: MultiAsset = (Parent, amount).into();

	let limit: u64 = match junctions {
		Here => 195952000 + 30,
		_ => 195952000 + 40,
	};

	Relay::execute_with(|| {
		let xcm = vec![
			WithdrawAsset(assets.clone().into()),
			BuyExecution {
				fees: assets,
				weight_limit: Limited(limit),
			},
			Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: 195952000 as u64,
				call: call.encode().into(),
			},
		];
		assert_ok!(RelayChainPalletXcm::send_xcm(junctions, Parachain(1).into(), Xcm(xcm),));
	});
}

fn parachain_transact_to_relaychian_remark() {
	use relay::{Call, Runtime};
	let call = Call::System(frame_system::Call::<Runtime>::remark_with_event { remark: vec![1, 2, 3] });
	let assets: MultiAsset = (Here, 6030).into();

	assert_ok!(ParachainPalletXcm::send_xcm(
		Here,
		Parent,
		Xcm(vec![
			WithdrawAsset(assets.clone().into()),
			BuyExecution {
				fees: assets,
				weight_limit: Limited(6030)
			},
			Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: 6000 as u64,
				call: call.encode().into(),
			},
		]),
	));
}

fn parachain_transact_to_sibling_remark(junctions: Junctions, amount: u128) {
	use relay::{Call, Runtime};
	let call = Call::System(frame_system::Call::<Runtime>::remark_with_event { remark: vec![1, 2, 3] });
	let assets: MultiAsset = (Parent, amount).into();
	let limit: u64 = match junctions {
		Here => 6030,
		_ => 6040,
	};

	ParaA::execute_with(|| {
		let xcm = vec![
			WithdrawAsset(assets.clone().into()),
			BuyExecution {
				fees: assets,
				weight_limit: Limited(limit),
			},
			Transact {
				origin_type: OriginKind::SovereignAccount,
				require_weight_at_most: 6000 as u64,
				call: call.encode().into(),
			},
		];

		assert_ok!(ParachainPalletXcm::send_xcm(
			junctions,
			(Parent, Parachain(2)),
			Xcm(xcm)
		));
	});
}

#[test]
fn call_size_limit() {
	// Ensures Call enum doesn't allocate more than 200 bytes in runtime
	assert!(
		core::mem::size_of::<crate::Call::<crate::tests::para::Runtime>>() <= 200,
		"size of Call is more than 200 bytes: some calls have too big arguments, use Box to \
		reduce the size of Call.
		If the limit is too strong, maybe consider increasing the limit",
	);

	assert!(
		core::mem::size_of::<orml_xcm::Call::<crate::tests::para::Runtime>>() <= 200,
		"size of Call is more than 200 bytes: some calls have too big arguments, use Box to \
		reduce the size of Call.
		If the limit is too strong, maybe consider increasing the limit",
	);
}
