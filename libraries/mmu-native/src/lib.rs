#![no_std]
#![feature(const_trait_impl, const_deref)]

extern crate alloc;

#[allow(unused_imports)]
pub use arch::*;
#[allow(unused_imports)]
pub use pte::{IArchPageTableEntry, IArchPageTableEntryBase};

mod arch;
mod pte;

#[cfg(target_os = "none")]
mod pt;

#[cfg(target_os = "none")]
pub use pt::*;

#[cfg(all(target_arch = "riscv64", target_os = "none"))]
pub type PageTable = PageTableNative<SV39PageTableAttribute, RV64PageTableEntry>;
