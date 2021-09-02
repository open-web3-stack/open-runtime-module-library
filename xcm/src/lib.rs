//! # Xcm Module

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::large_enum_variant)]

use frame_support::{pallet_prelude::*, traits::EnsureOrigin};
use frame_system::pallet_prelude::*;
use sp_std::boxed::Box;

use xcm::v0::prelude::*;

pub use module::*;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_xcm::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The required origin for sending XCM as parachain sovereign.
		///
		/// Typically root or the majority of collective.
		type SovereignOrigin: EnsureOrigin<Self::Origin>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// XCM message sent. \[from, to, message\]
		Sent(MultiLocation, MultiLocation, Xcm<()>),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The message and destination combination was not recognized as being
		/// reachable.
		Unreachable,
		/// The message and destination was recognized as being reachable but
		/// the operation could not be completed.
		SendFailure,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Send an XCM message as parachain sovereign.
		#[pallet::weight(100_000_000)]
		pub fn send_as_sovereign(
			origin: OriginFor<T>,
			dest: Box<MultiLocation>,
			message: Box<Xcm<()>>,
		) -> DispatchResult {
			let _ = T::SovereignOrigin::ensure_origin(origin)?;
			pallet_xcm::Pallet::<T>::send_xcm(MultiLocation::Null, *dest.clone(), *message.clone()).map_err(
				|e| match e {
					XcmError::CannotReachDestination(..) => Error::<T>::Unreachable,
					_ => Error::<T>::SendFailure,
				},
			)?;
			Self::deposit_event(Event::Sent(MultiLocation::Null, *dest, *message));
			Ok(())
		}
	}
}
