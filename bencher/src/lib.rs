#![cfg_attr(not(feature = "std"), no_std)]

#[doc(hidden)]
pub extern crate codec;
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
pub extern crate sc_executor;
#[doc(hidden)]
#[cfg(feature = "std")]
pub extern crate sp_io;

#[cfg(feature = "std")]
mod run_benches;
