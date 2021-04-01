//! # Xtokens Module
//!
//! ## Overview
//!
//! The xtokens module provides cross-chain token transfer functionality, by
//! cross-consensus messages(XCM).
//!
//! The xtokens module provides functions for
//! - Token transfer from parachains to relay chain.
//! - Token transfer between parachains, including relay chain tokens like DOT,
//!   KSM, and parachain tokens like ACA, aUSD.
//!
//! ## Interface
//!
//! ### Dispatchable functions
//!
//! - `transfer_to_relay_chain`: Transfer relay chain tokens to relay chain.
//! - `transfer_to_parachain`: Transfer tokens to a sibling parachain.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::from_over_into)]
#![allow(clippy::unused_unit)]
#![allow(clippy::large_enum_variant)]

mod reserve_location;

pub use module::*;
pub use reserve_location::*;

#[frame_support::pallet]
pub mod module {
	use super::*;

	use frame_support::{pallet_prelude::*, traits::Get, transactional, Parameter};
	use frame_system::{ensure_signed, pallet_prelude::*};
	use sp_runtime::traits::{AtLeast32BitUnsigned, Convert, MaybeSerializeDeserialize, Member};
	use sp_std::prelude::*;

	use cumulus_primitives_core::relay_chain::Balance as RelayChainBalance;
	use xcm::v0::{
		MultiAsset, MultiLocation, Order,
		Order::*,
		Xcm::{self, *},
	};

	use orml_xcm_support::XcmHandler;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The balance type.
		type Balance: Parameter
			+ Member
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ Into<u128>;

		/// Convert `Balance` to `RelayChainBalance`.
		type ToRelayChainBalance: Convert<Self::Balance, RelayChainBalance>;

		/// Convert `Self::Account` to `AccountId32`
		type AccountId32Convert: Convert<Self::AccountId, [u8; 32]>;

		/// Self chain location.
		#[pallet::constant]
		type SelfLocation: Get<MultiLocation>;

		/// Xcm handler to execute XCM.
		type XcmHandler: XcmHandler<Self::AccountId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	pub enum Event<T: Config> {
		/// Transferred `MultiAsset`. \[sender, asset, dest, recipient\]
		TransferredMultiAsset(T::AccountId, MultiAsset, MultiLocation, MultiLocation),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Asset has no reserve location.
		AssetHasNoReserve,
		/// Not cross-chain transfer.
		NotCrossChainTransfer,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Transfer `asset` to `recipient` in `dest` chain.
		#[transactional]
		#[pallet::weight(1000)]
		pub fn transfer_multiasset(
			origin: OriginFor<T>,
			asset: MultiAsset,
			dest: MultiLocation,
			recipient: MultiLocation,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			Self::do_transfer_multiasset(who.clone(), asset.clone(), dest.clone(), recipient.clone())?;
			Self::deposit_event(Event::<T>::TransferredMultiAsset(who, asset, dest, recipient));
			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Transfer `MultiAsset` without depositing event.
		fn do_transfer_multiasset(
			who: T::AccountId,
			asset: MultiAsset,
			dest: MultiLocation,
			recipient: MultiLocation,
		) -> DispatchResultWithPostInfo {
			let self_location = T::SelfLocation::get();
			ensure!(dest != self_location, Error::<T>::NotCrossChainTransfer);

			let reserve = asset.reserve().ok_or(Error::<T>::AssetHasNoReserve)?;
			let xcm = if reserve == self_location {
				Self::transfer_self_reserve_asset(asset, dest, recipient)
			} else if reserve == dest {
				Self::transfer_to_reserve(asset, dest, recipient)
			} else {
				Self::transfer_to_non_reserve(asset, reserve, dest, recipient)
			};

			T::XcmHandler::execute_xcm(who.clone(), xcm)?;

			Ok(().into())
		}

		fn transfer_self_reserve_asset(asset: MultiAsset, dest: MultiLocation, recipient: MultiLocation) -> Xcm {
			WithdrawAsset {
				assets: vec![asset],
				effects: vec![DepositReserveAsset {
					assets: vec![MultiAsset::All],
					dest,
					effects: Self::deposit_asset(recipient),
				}],
			}
		}

		fn transfer_to_reserve(asset: MultiAsset, reserve: MultiLocation, recipient: MultiLocation) -> Xcm {
			WithdrawAsset {
				assets: vec![asset],
				effects: vec![InitiateReserveWithdraw {
					assets: vec![MultiAsset::All],
					reserve,
					effects: Self::deposit_asset(recipient),
				}],
			}
		}

		fn transfer_to_non_reserve(
			asset: MultiAsset,
			reserve: MultiLocation,
			dest: MultiLocation,
			recipient: MultiLocation,
		) -> Xcm {
			WithdrawAsset {
				assets: vec![asset],
				effects: vec![InitiateReserveWithdraw {
					assets: vec![MultiAsset::All],
					reserve,
					effects: vec![DepositReserveAsset {
						assets: vec![MultiAsset::All],
						dest,
						effects: Self::deposit_asset(recipient),
					}],
				}],
			}
		}

		fn deposit_asset(recipient: MultiLocation) -> Vec<Order> {
			vec![DepositAsset {
				assets: vec![MultiAsset::All],
				dest: recipient,
			}]
		}
	}
}
