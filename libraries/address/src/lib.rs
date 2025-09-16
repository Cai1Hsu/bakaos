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
#[macro_use]
pub(crate) mod page_base;
#[macro_use]
pub(crate) mod page_range_base;

mod phys_addr;
mod phys_addr_range;
mod phys_page;
mod phys_page_range;

mod virt_addr;
mod virt_addr_range;
mod virt_page;
mod virt_page_range;

pub use phys_addr::PhysAddr;
pub use phys_addr_range::PhysAddrRange;
pub use phys_page::PhysPage;
pub use phys_page_range::PhysPageRange;

pub use virt_addr::VirtAddr;
pub use virt_addr_range::VirtAddrRange;
pub use virt_page::VirtPage;
pub use virt_page_range::VirtPageRange;

pub mod virt {
    pub use super::virt_addr_range::RangeIterator as AddrIterator;
    pub use super::virt_page_range::RangeIterator as PageIterator;
}

pub mod phys {
    pub use super::phys_addr_range::RangeIterator as AddrIterator;
    pub use super::phys_page_range::RangeIterator as PageIterator;
}
