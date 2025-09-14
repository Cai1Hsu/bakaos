use crate::VirtAddr;

impl_range!(VaddrRange, VirtAddr,
    /// Represents a range of virtual addresses.
    ///
    /// A virtual address range is defined by a start and end address,
    /// where the start is inclusive and the end is exclusive.
    /// This is commonly used for memory mapping, virtual memory management,
    /// and user space address ranges.
);

#[cfg(test)]
mod virt_range_tests {
    use super::*;

    #[test]
    fn test_virt_range_creation() {
        let start = VirtAddr::new(0x400000);
        let end = VirtAddr::new(0x500000);
        let range = VaddrRange::new(start, end);

        assert_eq!(range.start(), start);
        assert_eq!(range.end(), end);
        assert_eq!(range.len(), 0x100000);
    }

    #[test]
    fn test_virt_range_page_table_operations() {
        // Test with different page sizes common in virtual memory
        let range = VaddrRange::new(VirtAddr::new(0x400000), VirtAddr::new(0x800000));

        // 4KB pages
        let pages_4k: Vec<_> = range.iter_pages().unwrap().collect();
        assert_eq!(pages_4k.len(), (0x800000 - 0x400000) / 0x1000);

        // 2MB pages
        let pages_2m: Vec<_> = range.iter_pages_sized(0x200000).unwrap().collect();
        assert_eq!(pages_2m.len(), (0x800000 - 0x400000) / 0x200000);

        // 1GB pages
        assert!(range.iter_pages_sized(0x40000000).is_none()); // smaller than 1GB, and length not multiple of 1GB

        let pages_1g: Vec<_> = RangeIterator::new_unchecked(range, 0x40000000).collect();
        assert_eq!(pages_1g.len(), 0); // Range is smaller than 1GB
    }

    #[test]
    fn test_virt_range_from_pointers() {
        // Test conversion from Rust references/pointers
        let data = [1u32, 2, 3, 4, 5];
        let ptr_start = data.as_ptr();
        let ptr_end = unsafe { ptr_start.add(data.len()) };

        let start_addr = VirtAddr::from(ptr_start);
        let end_addr = VirtAddr::from(ptr_end);
        let range = VaddrRange::new(start_addr, end_addr);

        assert_eq!(range.len(), data.len() * core::mem::size_of::<u32>());
    }

    #[test]
    fn test_virt_range_alignment_for_pages() {
        // Test alignment operations common in virtual memory management
        let unaligned_range = VaddrRange::new(VirtAddr::new(0x401234), VirtAddr::new(0x405678));

        // Align to 4KB pages
        let page_aligned = unaligned_range.align_to(0x1000);
        assert!(page_aligned.start().is_aligned(0x1000));
        assert!(page_aligned.end().is_aligned(0x1000));
        assert_eq!(page_aligned.start(), VirtAddr::new(0x401000));
        assert_eq!(page_aligned.end(), VirtAddr::new(0x406000));

        // Align to 2MB pages
        let huge_page_aligned = unaligned_range.align_to(0x200000);
        assert!(huge_page_aligned.start().is_aligned(0x200000));
        assert!(huge_page_aligned.end().is_aligned(0x200000));
        assert_eq!(huge_page_aligned.start(), VirtAddr::new(0x400000));
        assert_eq!(huge_page_aligned.end(), VirtAddr::new(0x600000));
    }

    #[test]
    fn test_virt_range_intersection_for_protection() {
        // Test scenarios common in memory protection
        let executable_range = VaddrRange::new(VirtAddr::new(0x400000), VirtAddr::new(0x500000));

        let writable_range = VaddrRange::new(VirtAddr::new(0x450000), VirtAddr::new(0x550000));

        // Find overlapping region (would need special protection)
        let overlap = executable_range.intersection(writable_range).unwrap();
        assert_eq!(overlap.start(), VirtAddr::new(0x450000));
        assert_eq!(overlap.end(), VirtAddr::new(0x500000));

        // This overlap would typically be invalid (W^X policy)
        assert_eq!(overlap.len(), 0xB0000);
    }

    #[test]
    fn test_virt_range_merge_adjacent_mappings() {
        // Test merging adjacent virtual memory mappings
        let mapping1 = VaddrRange::new(VirtAddr::new(0x600000), VirtAddr::new(0x700000));
        let mapping2 = VaddrRange::new(VirtAddr::new(0x700000), VirtAddr::new(0x800000));
        let mapping3 = VaddrRange::new(VirtAddr::new(0x900000), VirtAddr::new(0xA00000));

        // Adjacent mappings can be merged
        let merged = mapping1.merge(mapping2).unwrap();
        assert_eq!(merged.start(), VirtAddr::new(0x600000));
        assert_eq!(merged.end(), VirtAddr::new(0x800000));

        // Non-adjacent cannot be merged
        assert!(mapping1.merge(mapping3).is_none());
    }

    #[test]
    fn test_virt_range_contains_specific_addresses() {
        let stack_range = VaddrRange::new(
            VirtAddr::new(0x7FFE_0000_0000),
            VirtAddr::new(0x7FFF_0000_0000),
        );

        // Test stack pointer addresses
        let stack_ptr = VirtAddr::new(0x7FFE_8000_0000);
        assert!(stack_range.contains_addr(stack_ptr));

        // Test invalid stack addresses
        let invalid_stack = VirtAddr::new(0x7FFF_8000_0000);
        assert!(!stack_range.contains_addr(invalid_stack));
    }
}
