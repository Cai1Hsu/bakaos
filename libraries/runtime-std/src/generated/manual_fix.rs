pub use ::core::{
    assert, assert_eq, assert_ne, cfg, column, compile_error, concat, debug_assert,
    debug_assert_eq, debug_assert_ne, env, file, format_args, include, include_bytes, include_str,
    line, matches, module_path, option_env, stringify, todo, unimplemented, unreachable, write,
    writeln,
};

#[cfg(feature = "alloc")]
pub use ::alloc::{format, vec};

pub mod prelude {
    pub mod v1 {
        // Prelude
        #[cfg(all(feature = "alloc", not(feature = "unstable")))]
        pub use ::alloc::{};
        pub use ::core::prelude::rust_2021::*;

        // Other imports
        #[cfg(feature = "alloc")]
        pub use ::alloc::{
            borrow::ToOwned, boxed::Box, format, string::String, string::ToString, vec, vec::Vec,
        };
    }
}

pub mod os {
    pub mod raw {
        pub use ::core::ffi::c_void;

        #[cfg(feature = "libc")]
        pub use libc::{
            c_char, c_double, c_float, c_int, c_long, c_longlong, c_schar, c_short, c_uchar,
            c_uint, c_ulong, c_ulonglong, c_ushort,
        };
    }
}
