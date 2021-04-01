#![cfg_attr(not(feature = "std"), no_std)]

pub extern crate codec;
pub extern crate sp_core;
pub extern crate sp_runtime_interface;
pub extern crate sp_std;

mod bench;

#[cfg(feature = "std")]
pub extern crate linregress;
#[cfg(feature = "std")]
pub extern crate sp_io;

#[cfg(feature = "std")]
mod run_benchmarks;
