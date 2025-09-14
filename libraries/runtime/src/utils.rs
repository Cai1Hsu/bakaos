pub use ::utilities::*; // reexport `utilities`` crate

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
        unsafe extern "C" {
            #[doc(hidden)]
            #[allow(improper_ctypes)]
            #[link_name = $sym]
            static __SYM: ();
        }

        // Accessing the pointer to a symbol is unsafe,
        // We use a `unsafe` function, so that the caller must call this macro within an `unsafe` block.
        #[doc(hidden)]
        #[inline(always)] // by utilizing inline and const fn, we can avoid any runtime overhead
        const unsafe fn __get_sym() -> *const () {
            ::core::ptr::addr_of!(__SYM)
        }

        __get_sym()
    }};
    ($sym:literal as $t:ty) => {{
        ::core::ptr::NonNull::new_unchecked($crate::symbol_ptr!($sym) as *mut $t)
    }};
}

#[cfg(test)]
mod tests {
    use core::ptr::NonNull;

    #[unsafe(export_name = "__test_symbol")]
    static TEST_SYMBOL: u8 = 0;

    const P1: *const () = unsafe { symbol_ptr!("__test_symbol") };
    const P2: NonNull<()> = unsafe { symbol_ptr!("__test_symbol" as ()) };
    const P3: NonNull<u8> = unsafe { symbol_ptr!("__test_symbol" as u8) };

    #[test]
    fn test_symbol_ptr_same_address() {
        let expected = &TEST_SYMBOL as *const u8 as usize;

        let p1 = P1 as usize;
        let p2 = P2.as_ptr() as usize;
        let p3 = P3.as_ptr() as usize;

        assert_eq!(expected, p1);
        assert_eq!(expected, p2);
        assert_eq!(expected, p3);

        let _ = TEST_SYMBOL; // keep symbol from being optimized out
    }
}
