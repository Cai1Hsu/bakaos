use crate::PhysAddr;

impl_range!(PaddrRange, PhysAddr,
    /// Represents a range of physical addresses.
    ///
    /// A physical address range is defined by a start and end address,
    /// where the start is inclusive and the end is exclusive.
);

#[cfg(test)]
mod phys_range_tests {
    use super::*;

    #[test]
    fn test_phys_range_creation() {
        let start = PhysAddr::new(0x1000);
        let end = PhysAddr::new(0x2000);
        let range = PaddrRange::new(start, end);

        assert_eq!(range.start(), start);
        assert_eq!(range.end(), end);
        assert_eq!(range.len(), 0x1000);
    }

    #[test]
    fn test_phys_range_page_operations() {
        // Test with 4KB pages
        let page_size = 4096usize;
        let unaligned_start = PhysAddr::new(0x1234);
        let unaligned_end = PhysAddr::new(0x2345);
        let range = PaddrRange::new(unaligned_start, unaligned_end);

        let page_aligned = range.align_to(page_size);
        assert_eq!(page_aligned.start(), PhysAddr::new(0x1000));
        assert_eq!(page_aligned.end(), PhysAddr::new(0x3000));

        // Test page iteration
        let pages: Vec<_> = page_aligned.iter_pages().unwrap().collect();
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[0], PhysAddr::new(0x1000));
        assert_eq!(pages[1], PhysAddr::new(0x2000));
    }

    #[test]
    fn test_phys_range_memory_operations() {
        // Test ranges that might represent memory regions
        let dram_range = PaddrRange::new(PhysAddr::new(0x80000000), PhysAddr::new(0x100000000));
        let device_range = PaddrRange::new(PhysAddr::new(0x10000000), PhysAddr::new(0x20000000));

        // They shouldn't overlap
        assert!(!dram_range.overlaps(device_range));

        // Test memory region size calculations
        assert_eq!(dram_range.len(), 0x80000000); // 2GB
        assert_eq!(device_range.len(), 0x10000000); // 256MB
    }

    #[test]
    fn test_phys_range_merge_adjacent_pages() {
        let page1 = PaddrRange::new(PhysAddr::new(0x1000), PhysAddr::new(0x2000));
        let page2 = PaddrRange::new(PhysAddr::new(0x2000), PhysAddr::new(0x3000));
        let page3 = PaddrRange::new(PhysAddr::new(0x3000), PhysAddr::new(0x4000));

        // Merge adjacent pages
        let merged12 = page1.merge(page2).unwrap();
        assert_eq!(merged12.start(), PhysAddr::new(0x1000));
        assert_eq!(merged12.end(), PhysAddr::new(0x3000));

        let merged_all = merged12.merge(page3).unwrap();
        assert_eq!(merged_all.start(), PhysAddr::new(0x1000));
        assert_eq!(merged_all.end(), PhysAddr::new(0x4000));
        assert_eq!(merged_all.len(), 0x3000);
    }

    #[test]
    fn test_phys_range_intersection() {
        let range1 = PaddrRange::new(PhysAddr::new(0x1000), PhysAddr::new(0x3000));
        let range2 = PaddrRange::new(PhysAddr::new(0x2000), PhysAddr::new(0x4000));

        let intersection = range1.intersection(range2).unwrap();
        assert_eq!(intersection.start(), PhysAddr::new(0x2000));
        assert_eq!(intersection.end(), PhysAddr::new(0x3000));
        assert_eq!(intersection.len(), 0x1000);
    }

    #[test]
    fn test_phys_range_large_addresses() {
        // Test with 64-bit physical addresses
        let high_range = PaddrRange::new(
            PhysAddr::new(0xFFFF_0000_0000_0000),
            PhysAddr::new(0xFFFF_0000_1000_0000),
        );

        assert_eq!(high_range.len(), 0x1000_0000);
        assert!(high_range.contains_addr(PhysAddr::new(0xFFFF_0000_0800_0000)));
        assert!(!high_range.contains_addr(PhysAddr::new(0xFFFF_0000_2000_0000)));
    }

    #[test]
    fn test_phys_range_empty_and_edge_cases() {
        let addr = PhysAddr::new(0x1000);
        let empty_range = PaddrRange::new(addr, addr);

        assert!(empty_range.is_empty());
        assert_eq!(empty_range.len(), 0);
        assert!(!empty_range.contains_addr(addr)); // empty range contains nothing

        // Test minimum range
        let min_range = PaddrRange::new(PhysAddr::new(0x1000), PhysAddr::new(0x1001));
        assert_eq!(min_range.len(), 1);
        assert!(min_range.contains_addr(PhysAddr::new(0x1000)));
        assert!(!min_range.contains_addr(PhysAddr::new(0x1001)));
    }

    #[test]
    fn test_phys_range_iterator_exact_size() {
        let range = PaddrRange::new(PhysAddr::new(0x1000), PhysAddr::new(0x1010));
        let iter = range.iter_step(4).unwrap();

        // Test ExactSizeIterator
        assert_eq!(iter.len(), 4);

        let collected: Vec<_> = iter.collect();
        assert_eq!(collected.len(), 4);
    }
}
