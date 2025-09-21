use core::ops::Range;

/// Linear mapping window for physical memory
pub const LINEAR_WINDOW: Range<usize> = 0xffff_ffc0_0000_0000..usize::MAX; // TODO: use a more reasonable upper bound

/// Check if a virtual address is within the linear mapping window
pub const fn is_linear_window(vaddr: usize) -> bool {
    LINEAR_WINDOW.start <= vaddr && vaddr < LINEAR_WINDOW.end
}

/// Get the corresponding virtual address in the linear mapping window for a given physical address
pub const fn get_linear_vaddr(paddr: usize) -> usize {
    debug_assert!(!is_linear_window(paddr));

    paddr + LINEAR_WINDOW.start
}
