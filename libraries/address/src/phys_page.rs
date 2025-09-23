use crate::{PhysAddr, PhysAddrRange};

impl_page!(PhysPage, PhysAddr, PhysAddrRange,
    /// Represents a physical memory page, or frame number (PFN).
    ///
    /// A physical page is defined by its starting physical address and size.
    /// Common page sizes include 4KB, 2MB, and 1GB.
    /// This is commonly used in physical memory allocation, page table entries,
    /// and memory mapping.
);
