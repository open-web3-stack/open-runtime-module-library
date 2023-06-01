//! # Xcm Module

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::large_enum_variant)]

use frame_support::{pallet_prelude::*, traits::EnsureOrigin};
use frame_system::pallet_prelude::*;
use sp_std::boxed::Box;
use xcm::{v3::prelude::*, VersionedMultiLocation, VersionedXcm};

pub use module::*;

#[frame_support::pallet]
pub mod module {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_xcm::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The required origin for sending XCM as parachain sovereign.
		///
		/// Typically root or the majority of collective.
		type SovereignOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// XCM message sent. \[to, message\]
		Sent { to: MultiLocation, message: Xcm<()> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The message and destination combination was not recognized as being
		/// reachable.
		Unreachable,
		/// The message and destination was recognized as being reachable but
		/// the operation could not be completed.
		SendFailure,
		/// The version of the `Versioned` value used is not able to be
		/// interpreted.
		BadVersion,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Send an XCM message as parachain sovereign.
		#[pallet::call_index(0)]
		// FIXME: Benchmark send
		#[pallet::weight(Weight::from_parts(100_000_000, 0))]
		pub fn send_as_sovereign(
			origin: OriginFor<T>,
			dest: Box<VersionedMultiLocation>,
			message: Box<VersionedXcm<()>>,
		) -> DispatchResult {
			let _ = T::SovereignOrigin::ensure_origin(origin)?;
			let dest = MultiLocation::try_from(*dest).map_err(|()| Error::<T>::BadVersion)?;
			let message: Xcm<()> = (*message).try_into().map_err(|()| Error::<T>::BadVersion)?;

			pallet_xcm::Pallet::<T>::send_xcm(Here, dest, message.clone()).map_err(|e| match e {
				SendError::Unroutable => Error::<T>::Unreachable,
				_ => Error::<T>::SendFailure,
			})?;
			Self::deposit_event(Event::Sent { to: dest, message });
			Ok(())
		}
	}
}
