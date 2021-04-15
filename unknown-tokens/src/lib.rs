#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use frame_support::pallet_prelude::*;
use sp_std::vec::Vec;
use xcm::v0::{MultiAsset, MultiLocation};

use orml_xcm_support::UnknownAsset;

pub use module::*;

mod mock;
mod tests;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event {
		/// Deposit success. [asset, to]
		Deposited(MultiAsset, MultiLocation),
		/// Withdraw success. [asset, from]
		Withdrawn(MultiAsset, MultiLocation),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The balance is too low.
		BalanceTooLow,
		/// The operation will cause balance to overflow.
		BalanceOverflow,
		/// Unhandled asset.
		UnhandledAsset,
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

	/// Concrete fungible balances under a given location and a concrete
	/// fungible id.
	///
	/// double_map: who, asset_id => u128
	#[pallet::storage]
	#[pallet::getter(fn concrete_fungible_balances)]
	pub(crate) type ConcreteFungibleBalances<T> =
		StorageDoubleMap<_, Blake2_128Concat, MultiLocation, Blake2_128Concat, MultiLocation, u128, ValueQuery>;

	/// Abstract fungible balances under a given location and a abstract
	/// fungible id.
	///
	/// double_map: who, asset_id => u128
	#[pallet::storage]
	#[pallet::getter(fn abstract_fungible_balances)]
	pub(crate) type AbstractFungibleBalances<T> =
		StorageDoubleMap<_, Blake2_128Concat, MultiLocation, Blake2_128Concat, Vec<u8>, u128, ValueQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {}
}

impl<T: Config> UnknownAsset for Pallet<T> {
	fn deposit(asset: &MultiAsset, to: &MultiLocation) -> DispatchResult {
		match asset {
			MultiAsset::ConcreteFungible { id, amount } => {
				ConcreteFungibleBalances::<T>::try_mutate(to, id, |b| -> DispatchResult {
					*b = b.checked_add(*amount).ok_or(Error::<T>::BalanceOverflow)?;
					Ok(())
				})
			}
			MultiAsset::AbstractFungible { id, amount } => {
				AbstractFungibleBalances::<T>::try_mutate(to, id, |b| -> DispatchResult {
					*b = b.checked_add(*amount).ok_or(Error::<T>::BalanceOverflow)?;
					Ok(())
				})
			}
			_ => Err(Error::<T>::UnhandledAsset.into()),
		}?;

		Self::deposit_event(Event::Deposited(asset.clone(), to.clone()));

		Ok(())
	}

	fn withdraw(asset: &MultiAsset, from: &MultiLocation) -> DispatchResult {
		match asset {
			MultiAsset::ConcreteFungible { id, amount } => {
				ConcreteFungibleBalances::<T>::try_mutate(from, id, |b| -> DispatchResult {
					*b = b.checked_sub(*amount).ok_or(Error::<T>::BalanceTooLow)?;
					Ok(())
				})
			}
			MultiAsset::AbstractFungible { id, amount } => {
				AbstractFungibleBalances::<T>::try_mutate(from, id, |b| -> DispatchResult {
					*b = b.checked_sub(*amount).ok_or(Error::<T>::BalanceTooLow)?;
					Ok(())
				})
			}
			_ => Err(Error::<T>::UnhandledAsset.into()),
		}?;

		Self::deposit_event(Event::Withdrawn(asset.clone(), from.clone()));

		Ok(())
	}
}
