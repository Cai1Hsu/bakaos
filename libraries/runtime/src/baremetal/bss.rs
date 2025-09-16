use crate::symbol_ptr;
use core::ptr::NonNull;

pub(crate) fn clear_bss() {
    unsafe { clear_bss_range(symbol_ptr!("__sbss").cast(), symbol_ptr!("__ebss").cast()) }
}

/// Converts a pointer to a per-CPU local storage address.
///
/// # Safety
///
/// - `ptr` must be a valid pointer within the CLS region
/// - The caller must ensure that __scls has been properly initialized
/// - The current CPU's CLS must be properly set up via store_tls_base
pub(crate) unsafe fn clear_bss_range(mut begin: NonNull<u8>, end: NonNull<u8>) {
    let begin = begin.as_mut();
    let len = end.as_ptr().offset_from(begin) as usize;

    core::ptr::write_bytes(begin, 0, len);
}
