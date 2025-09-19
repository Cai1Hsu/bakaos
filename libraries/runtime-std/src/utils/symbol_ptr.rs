//! This module provides a macro to get a pointer to a symbol defined in assembly or linker script.

/// Get a pointer to a symbol defined in assembly or linker script.
/// The symbol must be defined in the global scope.
/// The macro takes a string literal as input to support symbols with special characters (e.g. `$`, `@`, etc).
///
/// # Example
/// ```ignore
/// let start = symbol_ptr!("_start") as *const fn(usize) -> !;
/// let main = symbol_ptr!("main") as usize;
/// ```
/// # Safety
/// Accessing the pointer to a symbol is unsafe, `unsafe` is required to call this macro.
#[macro_export]
macro_rules! symbol_ptr {
    ($sym:literal) => {{
        // Accessing the pointer to a symbol is unsafe,
        // We use a `unsafe` function, so that the caller must call this macro within an `unsafe` block.
        #[doc(hidden)]
        #[inline(always)] // by utilizing inline and const fn, we can avoid any runtime overhead
        const unsafe fn __get_sym() -> ::core::ptr::NonNull<()> {
            unsafe extern "C" {
                #[doc(hidden)]
                #[allow(improper_ctypes)]
                #[link_name = $sym]
                static mut __SYM: ();
            }

            const PTR: *mut () = ::core::ptr::addr_of_mut!(__SYM);

            // null check at compile time if possible
            // This is useless most of the time, since a pointer from reference is always considered non-null by the compiler
            const _: () = assert!(!PTR.is_null(), "The symbol pointer must not be null");

            // SAFETY: PTR is not null, and the caller must ensure the symbol is valid for type $t
            #[allow(unused_unsafe)] // Rust 2024 migration
            unsafe { ::core::ptr::NonNull::new_unchecked(PTR) }
        }

        __get_sym()
    }};
}

#[cfg(test)]
mod tests {
    use core::ptr::NonNull;

    #[unsafe(export_name = "__test_symbol")]
    static TEST_SYMBOL: u8 = 42;

    const P1: *const () = unsafe { symbol_ptr!("__test_symbol").as_ptr() };
    const P2: NonNull<()> = unsafe { symbol_ptr!("__test_symbol").cast::<()>() };
    const P3: NonNull<u8> = unsafe { symbol_ptr!("__test_symbol").cast::<u8>() };

    #[test]
    fn test_symbol_ptr_same_address() {
        let expected = &TEST_SYMBOL as *const u8 as usize;

        let p1 = P1 as usize;
        let p2 = P2.as_ptr() as usize;
        let p3 = P3.as_ptr() as usize;
        let p4 = unsafe { symbol_ptr!("__test_symbol").as_ptr() as usize }; // resolved at runtime

        assert_eq!(expected, p1);
        assert_eq!(expected, p2);
        assert_eq!(expected, p3);
        assert_eq!(expected, p4);

        assert_eq!(unsafe { *P3.as_ptr() }, TEST_SYMBOL);

        let _ = TEST_SYMBOL; // keep symbol from being optimized out
    }
}
