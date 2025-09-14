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
pub(crate) mod addr_range_base;

mod phys_addr;
mod phys_addr_range;

mod virt_addr;
mod virt_addr_range;

pub use phys_addr::PhysAddr;
pub use phys_addr_range::PhysAddrRange;

pub use virt_addr::VirtAddr;
pub use virt_addr_range::VirtAddrRange;

pub mod virt {
    pub use super::virt_addr_range::RangeIterator as AddrRageIterator;
}

pub mod phys {
    pub use super::phys_addr_range::RangeIterator as AddrRageIterator;
}
