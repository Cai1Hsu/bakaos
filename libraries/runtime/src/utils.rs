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
#[macro_export]
macro_rules! symbol_ptr {
    ($sym:literal) => {{
        unsafe extern "C" {
            #[doc(hidden)]
            #[allow(improper_ctypes)]
            #[link_name = $sym]
            static __SYM: ();
        }

        ::core::ptr::addr_of!(__SYM)
    }};
    ($sym:literal as $t:ty) => {{
        #[allow(unused_unsafe)]
        unsafe {
            ::core::ptr::NonNull::new_unchecked($crate::symbol_ptr!($sym) as *mut $t)
        }
    }};
}

#[cfg(test)]
mod tests {
    use core::ptr::NonNull;

    #[unsafe(export_name = "__test_symbol")]
    static TEST_SYMBOL: u8 = 0;

    const P1: *const () = symbol_ptr!("__test_symbol");
    const P2: NonNull<()> = symbol_ptr!("__test_symbol" as ());
    const P3: NonNull<u8> = symbol_ptr!("__test_symbol" as u8);

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
