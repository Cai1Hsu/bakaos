use crate::{VirtAddrRange, VirtPage};

#[cfg(test)]
use crate::VirtAddr;

impl_page_range!(VirtPageRange, VirtPage, VirtAddr, VirtAddrRange,
    /// Represents a range of virtual memory pages.
    ///
    /// A page range is defined by a start page and a page count,
    /// where the start is inclusive and the range covers `page_count` pages.
    /// This is commonly used for memory mapping, page table management,
    /// and memory allocation.
);
