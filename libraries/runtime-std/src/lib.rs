#![no_std]
#![allow(internal_features)]
#![feature(core_intrinsics)]

// Custom library re-exports
pub mod std_compat;
pub mod utils;
pub use hermit_sync;

// Standard library re-exports

#[cfg(use_std)]
extern crate std;

#[cfg(use_std)]
pub use ::std::*;

#[cfg(not(use_std))]
pub use crate::std_compat::*;
