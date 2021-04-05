#![cfg_attr(not(feature = "std"), no_std)]

#[doc(hidden)]
pub extern crate codec;
#[doc(hidden)]
pub extern crate frame_benchmarking;
#[doc(hidden)]
pub extern crate sp_core;
#[doc(hidden)]
pub extern crate sp_runtime_interface;
#[doc(hidden)]
pub extern crate sp_std;

mod bench;

#[doc(hidden)]
#[cfg(feature = "std")]
pub extern crate linregress;
#[doc(hidden)]
#[cfg(feature = "std")]
pub extern crate sc_client_db;
#[doc(hidden)]
#[cfg(feature = "std")]
pub extern crate sc_executor;
#[doc(hidden)]
#[cfg(feature = "std")]
pub extern crate sp_io;
#[doc(hidden)]
#[cfg(feature = "std")]
pub extern crate sp_state_machine;

#[cfg(feature = "std")]
mod run_benches;
