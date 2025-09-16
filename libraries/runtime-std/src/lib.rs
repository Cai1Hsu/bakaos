#![no_std]
#![allow(internal_features)]
#![feature(core_intrinsics)]

// Custom library re-exports
pub mod std_compat;
pub mod utils;
pub use hermit_sync;

// Standard library re-exports

#[cfg(not(any(target_os = "none", feature = "no_std")))]
extern crate std;

#[cfg(not(any(target_os = "none", feature = "no_std")))]
pub use ::std::*;

#[cfg(any(target_os = "none", feature = "no_std"))]
pub use crate::std_compat::*;
