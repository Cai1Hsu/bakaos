#![cfg_attr(not(test), no_std)]
#![feature(const_cmp)]
#![feature(const_ops)]
#![feature(const_from)]
#![feature(const_deref)]
#![feature(const_default)]
#![feature(const_trait_impl)]
#![feature(specialization)]
#![allow(incomplete_features)]

#[macro_use]
pub(crate) mod addr_base;
#[macro_use]
pub(crate) mod range_base;

mod paddr_range;
mod phys_addr;
mod vaddr_range;
mod virt_addr;

pub use paddr_range::PaddrRange;
pub use phys_addr::PhysAddr;
pub use vaddr_range::VaddrRange;
pub use virt_addr::VirtAddr;

pub mod virt {
    pub use super::vaddr_range::RangeIterator;
}

pub mod phys {
    pub use super::paddr_range::RangeIterator;
}
