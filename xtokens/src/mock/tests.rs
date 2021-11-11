#![cfg(test)]

use super::*;
use super::para::AccountIdToMultiLocation;
use orml_traits::{MultiCurrency};
use xcm_simulator::TestExt;
use xcm_builder::{IsConcrete};
use xcm_executor::traits::MatchesFungible;

use crate::mock::relay::KsmLocation;
use crate::mock::para::RelayLocation;

#[test]
fn test_init_balance() {
    Relay::execute_with(|| {
        assert_eq!(RelayBalances::free_balance(&ALICE), INITIAL_BALANCE);
        assert_eq!(RelayBalances::free_balance(&BOB), 0);
        assert_eq!(RelayBalances::free_balance(&para_a_account()), 0);
        assert_eq!(RelayBalances::free_balance(&para_b_account()), 0);
    });

    ParaA::execute_with(|| {
        assert_eq!(ParaTokens::free_balance(CurrencyId::R, &ALICE), INITIAL_BALANCE);
        assert_eq!(ParaTokens::free_balance(CurrencyId::R, &BOB), 0);

        assert_eq!(ParaTokens::free_balance(CurrencyId::A, &ALICE), 0);
        assert_eq!(ParaTokens::free_balance(CurrencyId::B, &ALICE), 0);

        assert_eq!(ParaBalances::free_balance(&ALICE), 0);
        assert_eq!(ParaBalances::free_balance(&BOB), 0);
        assert_eq!(ParaBalances::free_balance(&sibling_b_account()), 0);
        assert_eq!(ParaBalances::free_balance(&sibling_c_account()), 0);
    });

    ParaB::execute_with(|| {
        assert_eq!(ParaTokens::free_balance(CurrencyId::R, &ALICE), INITIAL_BALANCE);
        assert_eq!(ParaTokens::free_balance(CurrencyId::R, &BOB), 0);
    });
}

#[test]
fn test_asset_matches_fungible() {
    // use raw way: VersionedMultiAssets -> MultiAssets -> Vec<MultiAsset>
    // `KsmLocation` in `relay.rs` is `Here`
    let assets: VersionedMultiAssets = (Here, 100u128).into();
    let assets: MultiAssets = assets.try_into().unwrap();
    let assets: Vec<MultiAsset> = assets.drain();
    for asset in assets {
        let assets: u128 = IsConcrete::<KsmLocation>::matches_fungible(&asset.clone()).unwrap_or_default();
        assert_eq!(assets, 100u128);
    }

    // use convenient way, `KsmLocation` in `relay.rs` is `Here`
    let asset: MultiAsset = (Here, 100u128).into();
    let amount: u128 = IsConcrete::<KsmLocation>::matches_fungible(&asset.clone()).unwrap_or_default();
    assert_eq!(amount, 100u128);

    // `KsmLocation` in `relay.rs` is `Here`
    let asset: MultiAsset = (X1(Parachain(1)), 100u128).into();
    let assets: u128 = IsConcrete::<KsmLocation>::matches_fungible(&asset.clone()).unwrap_or_default();
    assert_eq!(assets, 0);

    // `RelayLocation` in `para.rs` is `Parent`
    let asset: MultiAsset = (Parent, 100u128).into();
    let assets: u128 = IsConcrete::<RelayLocation>::matches_fungible(&asset.clone()).unwrap_or_default();
    assert_eq!(assets, 100);
}

#[test]
fn test_account_location_convert() {
    let account = Junction::AccountId32 {
        network: NetworkId::Any,
        id: ALICE.into()
    };

    let origin_location = AccountIdToMultiLocation::convert(ALICE);
    let junction: Junctions = origin_location.try_into().unwrap();
    assert_eq!(junction, X1(account.clone()));

    let parent: MultiLocation = Parent.into();
    assert_eq!(parent.parents, 1);
    assert_eq!(parent.interior, Here);
    assert_eq!(parent.contains_parents_only(1), true);

    let destination: MultiLocation = MultiLocation::new(
        1,
        X2(
            Parachain(2),
            account.clone(),
        )
    ).into();
    assert_eq!(destination.parents, 1);
    assert_eq!(destination.interior, X2(Parachain(2), account.clone()));

    let destination: MultiLocation = (
        Parent,
        Parachain(2),
        account.clone(),
    ).into();
    assert_eq!(destination.parents, 1);
    assert_eq!(destination.interior, X2(Parachain(2), account.clone()));

    let destination: MultiLocation = (
        Parent,
        account.clone()
    ).into();
    assert_eq!(destination.parents, 1);
    assert_eq!(destination.interior, X1(account.clone()));

    let destination: MultiLocation = (
        Parachain(2),
        account.clone()
    ).into();
    assert_eq!(destination.parents, 0);
    assert_eq!(destination.interior, X2(Parachain(2), account.clone()));

    let junction = X1(account.clone());
    let mut destination: MultiLocation = Parent.into();
    destination.append_with(junction).unwrap();
    assert_eq!(destination.parents, 1);
    assert_eq!(destination.interior, X1(account.clone()));
}

#[test]
fn test_parachain_convert_location_to_account() {
    use xcm_executor::traits::Convert;

    // ParentIsDefault
    let parent: MultiLocation = Parent.into();
    let account = para::LocationToAccountId::convert(parent);
    assert_eq!(account, Ok(GOD));

    // SiblingParachainConvertsVia
    let destination: MultiLocation = (
        Parent,
        Parachain(1),
    ).into();
    let account = para::LocationToAccountId::convert(destination);
    assert_eq!(account, Ok(sibling_a_account()));

    let alice = Junction::AccountId32 {
        network: NetworkId::Any,
        id: ALICE.into()
    };

    // AccountId32Aliases
    let destination: MultiLocation = (
        alice.clone()
    ).into();
    let account = para::LocationToAccountId::convert(destination);
    assert_eq!(account, Ok(ALICE));

    // RelaychainAccountId32Aliases
    let destination: MultiLocation = (
        Parent,
        alice.clone()
    ).into();
    let account = para::LocationToAccountId::convert(destination);
    assert_eq!(account, Ok(ALICE));

    // Error case 1: ../Parachain/Account
    let destination: MultiLocation = (
        Parent,
        Parachain(1),
        alice.clone()
    ).into();
    let account = para::LocationToAccountId::convert(destination.clone());
    assert_eq!(account, Err(destination));

    // Error case 2: ./Parachain
    let destination: MultiLocation = (
        Parachain(1),
    ).into();
    let account = para::LocationToAccountId::convert(destination.clone());
    assert_eq!(account, Err(destination));
}

#[test]
fn test_relaychain_convert_location_to_account() {
    use xcm_executor::traits::Convert;

    // ChildParachainConvertsVia
    let destination: MultiLocation = (
        Parachain(1),
    ).into();
    let account = relay::SovereignAccountOf::convert(destination);
    assert_eq!(account, Ok(para_a_account()));

    let alice = Junction::AccountId32 {
        network: NetworkId::Any,
        id: ALICE.into()
    };

    let alice_unknown_network = Junction::AccountId32 {
        network: NetworkId::Polkadot,
        id: ALICE.into()
    };

    // AccountId32Aliases
    let destination: MultiLocation = (
        alice.clone()
    ).into();
    let account = relay::SovereignAccountOf::convert(destination);
    assert_eq!(account, Ok(ALICE));

    // AccountId32Aliases with unknown-network location
    let destination: MultiLocation = (
        alice_unknown_network.clone()
    ).into();
    let account = relay::SovereignAccountOf::convert(destination.clone());
    assert_eq!(account, Err(destination));
}

#[test]
fn test_parachain_convert_origin() {
    use xcm_executor::traits::ConvertOrigin;

    let alice = Junction::AccountId32 {
        network: NetworkId::Any,
        id: ALICE.into()
    };

    // SovereignSignedViaLocation<ParentIsDefault>
    let destination: MultiLocation = Parent.into();
    let origin = para::XcmOriginToCallOrigin::convert_origin(
        destination.clone(), OriginKind::SovereignAccount);
    assert!(origin.is_ok());

    // SovereignSignedViaLocation<SiblingParachainConvertsVia>
    let destination: MultiLocation = (
        Parent,
        Parachain(1),
    ).into();
    let origin = para::XcmOriginToCallOrigin::convert_origin(destination.clone(), OriginKind::SovereignAccount);
    assert!(origin.is_ok());

    // SovereignSignedViaLocation<AccountId32Aliases>
    let destination: MultiLocation = (
        alice.clone()
    ).into();
    let origin = para::XcmOriginToCallOrigin::convert_origin(destination.clone(), OriginKind::SovereignAccount);
    assert!(origin.is_ok());

    // SovereignSignedViaLocation<RelaychainAccountId32Aliases>
    let destination: MultiLocation = (
        Parent,
        alice.clone()
    ).into();
    let origin = para::XcmOriginToCallOrigin::convert_origin(destination.clone(), OriginKind::SovereignAccount);
    assert!(origin.is_ok());

    // SovereignSignedViaLocation Error case
    let destination: MultiLocation = (
        Parent,
        Parachain(1),
        alice.clone()
    ).into();
    let origin = para::XcmOriginToCallOrigin::convert_origin(destination.clone(), OriginKind::SovereignAccount);
    assert!(origin.is_err());

    // RelayChainAsNative
    let destination: MultiLocation = Parent.into();
    let origin = para::XcmOriginToCallOrigin::convert_origin(
        destination.clone(), OriginKind::Native);
    assert!(origin.is_ok());

    // SiblingParachainAsNative
    let destination: MultiLocation = (
        Parent,
        Parachain(1),
    ).into();
    let origin = para::XcmOriginToCallOrigin::convert_origin(
        destination.clone(), OriginKind::Native);
    assert!(origin.is_ok());

    // SignedAccountId32AsNative
    let destination: MultiLocation = (
        alice.clone()
    ).into();
    let origin = para::XcmOriginToCallOrigin::convert_origin(
        destination.clone(), OriginKind::Native);
    assert!(origin.is_ok());

    // XcmPassthrough
    let destination: MultiLocation = (
        Parent,
        alice.clone()
    ).into();
    let origin = para::XcmOriginToCallOrigin::convert_origin(
        destination.clone(), OriginKind::Xcm);
    assert!(origin.is_ok());
}