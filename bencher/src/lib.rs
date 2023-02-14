#![cfg_attr(not(feature = "std"), no_std)]

#[doc(hidden)]
pub extern crate frame_support;
#[doc(hidden)]
pub extern crate paste;
#[doc(hidden)]
pub extern crate sp_core;
#[doc(hidden)]
pub extern crate sp_io;
#[doc(hidden)]
pub extern crate sp_std;

mod bencher;
mod macros;
mod utils;

pub use bencher::*;
pub use utils::*;

#[cfg(feature = "std")]
pub mod bench_runner;
#[cfg(feature = "std")]
pub mod build_wasm;
#[cfg(feature = "std")]
pub mod handler;

#[cfg(feature = "std")]
mod bench_ext;
#[cfg(feature = "std")]
mod colorize;
#[cfg(feature = "std")]
mod tracker;

pub use bencher_procedural::benchmarkable;
