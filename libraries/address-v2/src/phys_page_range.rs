use crate::{PhysAddrRange, PhysPage};

#[cfg(test)]
use crate::PhysAddr;

impl_page_range!(PhysPageRange, PhysPage, PhysAddr, PhysAddrRange,
    /// Represents a range of physical memory pages.
    ///
    /// A page range is defined by a start page and a page count,
    /// where the start is inclusive and the range covers `page_count` pages.
    /// This is commonly used for memory mapping, page table management,
    /// and memory allocation.
);
