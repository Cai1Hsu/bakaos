pub mod arch;
pub mod cpu;

#[cfg(all(target_os = "none", feature = "binary_crate"))]
mod panic;

#[cfg(target_os = "none")]
mod serial;

#[cfg(feature = "boot")]
pub(crate) mod bss;

#[cfg(feature = "boot")]
pub use boot_required::*;

#[cfg(feature = "boot")]
mod boot_required {
    use crate::{hermit_sync::SpinMutex, symbol_ptr};
    use core::{alloc::Layout, ptr::NonNull};

    static MEMORY_START: SpinMutex<usize> = SpinMutex::new(0);

    /// Do some arch-independent initialization of the memory region.
    ///
    /// # Safety
    ///
    /// This function must be called only once, before any memory allocation is performed.
    pub(crate) unsafe fn init() {
        *MEMORY_START.lock() = symbol_ptr!("__ekernel").as_ptr() as usize;
    }

    /// Returns the start address of the memory region.
    ///
    /// # Safety
    ///
    /// This function may only be called once the memory region initialization has been completed.
    pub unsafe fn memory_start() -> usize {
        *MEMORY_START.lock()
    }

    pub(crate) fn alloc_frame(layout: Layout) -> NonNull<u8> {
        let mut start = MEMORY_START.lock();

        let addr = start.next_multiple_of(layout.align());
        *start = addr + layout.size();

        NonNull::new(addr as *mut u8).unwrap()
    }
}
