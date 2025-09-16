#![no_std]
#![feature(linkage)]

extern crate runtime_std;

pub use runtime_std::*;

#[cfg(target_os = "none")]
pub mod baremetal;
mod hosted;

#[cfg(feature = "boot")]
mod entry;

#[cfg(feature = "boot")]
pub use entry::*;
