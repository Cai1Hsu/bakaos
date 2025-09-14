use crate::{VirtAddr, VirtAddrRange};

impl_page!(VirtPage, VirtAddr, VirtAddrRange,
    /// Represents a virtual memory page.
    ///
    /// A virtual page is defined by its starting virtual address and size.
    /// Common page sizes include 4KB, 2MB, and 1GB.
    /// This is commonly used in virtual memory management, page table entries,
    /// and memory mapping.
);
