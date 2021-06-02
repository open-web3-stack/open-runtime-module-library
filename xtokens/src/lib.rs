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
//! - `transfer`: Transfer local assets with given `CurrencyId` and `Amount`.
//! - `transfer_multiasset`: Transfer `MultiAsset` assets.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::from_over_into)]
#![allow(clippy::unused_unit)]
#![allow(clippy::large_enum_variant)]

use frame_support::{pallet_prelude::*, traits::Get, transactional, Parameter};
use frame_system::{ensure_signed, pallet_prelude::*};
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, Convert, MaybeSerializeDeserialize, Member, Zero},
	DispatchError,
};
use sp_std::prelude::*;

use xcm::v0::prelude::*;

use orml_traits::location::{Parse, Reserve};

// mod mock;
// mod tests;

pub use module::*;

#[frame_support::pallet]
pub mod module {
	use super::*;

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

		/// Currency Id.
		type CurrencyId: Parameter + Member + Clone;

		/// Convert `T::CurrencyId` to `MultiLocation`.
		type CurrencyIdConvert: Convert<Self::CurrencyId, Option<MultiLocation>>;

		/// Convert `T::AccountId` to `MultiLocation`.
		type AccountIdToMultiLocation: Convert<Self::AccountId, MultiLocation>;

		/// Self chain location.
		#[pallet::constant]
		type SelfLocation: Get<MultiLocation>;

		/// XCM executor.
		type XcmExecutor: ExecuteXcm<Self::Call>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId", T::CurrencyId = "CurrencyId", T::Balance = "Balance")]
	pub enum Event<T: Config> {
		/// Transferred. \[sender, currency_id, amount, dest\]
		Transferred(T::AccountId, T::CurrencyId, T::Balance, MultiLocation),
		/// Transferred `MultiAsset`. \[sender, asset, dest\]
		TransferredMultiAsset(T::AccountId, MultiAsset, MultiLocation),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Asset has no reserve location.
		AssetHasNoReserve,
		/// Not cross-chain transfer.
		NotCrossChainTransfer,
		/// Invalid transfer destination.
		InvalidDest,
		/// Currency is not cross-chain transferable.
		NotCrossChainTransferableCurrency,
		/// The message's weight could not be determined.
		UnweighableMessage,
		//TODO: more detailed err
		/// Xcm execution failed.
		XcmExecutionFailed,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Transfer native currencies.
		//TODO: weight
		#[transactional]
		#[pallet::weight(1000)]
		pub fn transfer(
			origin: OriginFor<T>,
			currency_id: T::CurrencyId,
			amount: T::Balance,
			dest: MultiLocation,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			if amount == Zero::zero() {
				return Ok(().into());
			}

			let id: MultiLocation = T::CurrencyIdConvert::convert(currency_id.clone())
				.ok_or(Error::<T>::NotCrossChainTransferableCurrency)?;
			let asset = MultiAsset::ConcreteFungible {
				id,
				amount: amount.into(),
			};
			Self::do_transfer_multiasset(who.clone(), asset, dest.clone())?;
			Self::deposit_event(Event::<T>::Transferred(who, currency_id, amount, dest));
			Ok(().into())
		}

		/// Transfer `MultiAsset`.
		//TODO: weight
		#[transactional]
		#[pallet::weight(1000)]
		pub fn transfer_multiasset(origin: OriginFor<T>, asset: MultiAsset, dest: MultiLocation) -> DispatchResult {
			let who = ensure_signed(origin)?;

			if Self::is_zero_amount(&asset) {
				return Ok(().into());
			}

			Self::do_transfer_multiasset(who.clone(), asset.clone(), dest.clone())?;
			Self::deposit_event(Event::<T>::TransferredMultiAsset(who, asset, dest));
			Ok(().into())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Transfer `MultiAsset` without depositing event.
		fn do_transfer_multiasset(who: T::AccountId, asset: MultiAsset, dest: MultiLocation) -> DispatchResult {
			let (dest, recipient) = Self::ensure_valid_dest(dest)?;

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

			//TODO: use weighter to get the actual weight
			let origin_location = T::AccountIdToMultiLocation::convert(who);
			let outcome = T::XcmExecutor::execute_xcm_in_credit(origin_location, xcm, 100_000_000_000, 100_000_000_000);
			match outcome {
				Outcome::Complete(_w) => Ok(().into()),
				//TODO: more detailed err
				Outcome::Incomplete(_w, _e) => Err(Error::<T>::XcmExecutionFailed.into()),
				//TODO: more detailed err
				Outcome::Error(_e) => Err(Error::<T>::XcmExecutionFailed.into()),
			}
		}

		fn transfer_self_reserve_asset(
			asset: MultiAsset,
			dest: MultiLocation,
			recipient: MultiLocation,
		) -> Xcm<T::Call> {
			//TODO: buy execution order
			WithdrawAsset {
				assets: vec![asset],
				effects: vec![DepositReserveAsset {
					assets: vec![MultiAsset::All],
					dest,
					effects: Self::deposit_asset(recipient),
				}],
			}
		}

		fn transfer_to_reserve(asset: MultiAsset, reserve: MultiLocation, recipient: MultiLocation) -> Xcm<T::Call> {
			//TODO: buy execution order
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
		) -> Xcm<T::Call> {
			//TODO: buy execution order
			let mut reanchored_dest = dest.clone();
			if reserve == Parent.into() {
				if let MultiLocation::X2(Parent, Parachain(id)) = dest {
					reanchored_dest = Parachain(id).into();
				}
			}

			WithdrawAsset {
				assets: vec![asset],
				effects: vec![InitiateReserveWithdraw {
					assets: vec![MultiAsset::All],
					reserve,
					effects: vec![DepositReserveAsset {
						assets: vec![MultiAsset::All],
						dest: reanchored_dest,
						effects: Self::deposit_asset(recipient),
					}],
				}],
			}
		}

		fn deposit_asset(recipient: MultiLocation) -> Vec<Order<()>> {
			vec![DepositAsset {
				assets: vec![MultiAsset::All],
				dest: recipient,
			}]
		}

		fn is_zero_amount(asset: &MultiAsset) -> bool {
			if let MultiAsset::ConcreteFungible { id: _, amount } = asset {
				if *amount == Zero::zero() {
					return true;
				}
			}

			if let MultiAsset::AbstractFungible { id: _, amount } = asset {
				if *amount == Zero::zero() {
					return true;
				}
			}

			false
		}

		/// Ensure has the `dest` has chain part and recipient part.
		fn ensure_valid_dest(
			dest: MultiLocation,
		) -> sp_std::result::Result<(MultiLocation, MultiLocation), DispatchError> {
			if let (Some(dest), Some(recipient)) = (dest.chain_part(), dest.non_chain_part()) {
				Ok((dest, recipient))
			} else {
				Err(Error::<T>::InvalidDest.into())
			}
		}
	}
}
