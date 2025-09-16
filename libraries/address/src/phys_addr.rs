impl_addr!(PhysAddr,
    /// Represents a physical address.
);

#[cfg(test)]
mod phys_addr_tests {
    use super::*;
    use crate::VirtAddr;

    #[test]
    fn test_phys_addr_creation() {
        let addr = PhysAddr::new(0x1000);
        assert_eq!(*addr, 0x1000);
        assert!(!addr.is_null());

        let null_addr = PhysAddr::null;
        assert!(null_addr.is_null());
        assert_eq!(*null_addr, 0);
    }

    #[test]
    fn test_phys_addr_arithmetic() {
        let addr1 = PhysAddr::new(0x1000);
        let addr2 = PhysAddr::new(0x2000);

        // Test physical address arithmetic
        assert_eq!(addr1 + 0x1000usize, addr2);
        assert_eq!(addr2 - 0x1000usize, addr1);
        assert_eq!(addr2 - addr1, 0x1000isize);

        // Test that physical addresses can represent page boundaries
        let page_size = 4096usize; // 4KB pages
        let page_addr = PhysAddr::new(0x10000); // Page-aligned
        let next_page = page_addr + page_size;
        assert_eq!(*next_page, 0x10000 + 4096);
    }

    #[test]
    fn test_phys_addr_page_alignment() {
        // Test common page sizes
        let page_4k = PhysAddr::new(0x1000); // 4KB aligned
        let page_2m = PhysAddr::new(0x200000); // 2MB aligned
        let page_1g = PhysAddr::new(0x40000000); // 1GB aligned

        assert_eq!(*page_4k & 0xFFF, 0); // 4KB aligned
        assert_eq!(*page_2m & 0x1FFFFF, 0); // 2MB aligned
        assert_eq!(*page_1g & 0x3FFFFFFF, 0); // 1GB aligned
    }

    #[test]
    fn test_phys_addr_high_memory() {
        // Test high physical memory addresses (important for 64-bit systems)
        let high_mem = PhysAddr::new(0xFFFF_FFFF_0000_0000);
        assert_eq!(*high_mem, 0xFFFF_FFFF_0000_0000);

        // Test that arithmetic works with high addresses
        let offset = high_mem + 0x1000usize;
        assert_eq!(*offset, 0xFFFF_FFFF_0000_1000);
    }

    #[test]
    fn test_phys_addr_conversions() {
        let addr = PhysAddr::new(0xDEADBEEF);

        // Test conversion to usize
        let addr_usize: usize = addr.into();
        assert_eq!(addr_usize, 0xDEADBEEF);

        // Test conversion from usize
        let addr_from: PhysAddr = PhysAddr::from(0xCAFEBABE);
        assert_eq!(*addr_from, 0xCAFEBABE);

        // Test that PhysAddr and VirtAddr are distinct types
        let vaddr = VirtAddr::new(0x1000);
        let paddr = PhysAddr::new(0x1000);

        // They should have the same underlying value but be different types
        assert_eq!(*vaddr, *paddr);

        // Test that they can't be directly compared (this would be a compile error)
        // assert_eq!(vaddr, paddr); // This should not compile
    }

    #[test]
    fn test_phys_addr_formatting() {
        let addr = PhysAddr::new(0x12345678);

        let debug_str = format!("{:?}", addr);
        assert_eq!(debug_str, "PhysAddr(0x12345678)");

        let display_str = format!("{}", addr);
        assert_eq!(display_str, "PhysAddr(0x12345678)");
    }
}
