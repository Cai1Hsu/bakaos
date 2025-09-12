/*
 * This library is derived from "no-std-compat"
 *   https://gitlab.com/jD91mZM2/no-std-compat
 * 
 * Original work Copyright (c) 2019 jD91mZM2
 * Licensed under the MIT License
 * See LICENSE in this directory(runtime-std).
 */

#![cfg_attr(not(feature = "std"), no_std)]
// TODO: handle unstable features

#[cfg(feature = "std")]
#[allow(unused_imports)]
pub mod prelude {
    pub mod v1 {
        pub use std::prelude::v1::*;
        // Macros aren't included in the prelude for some reason
        pub use std::{dbg, eprint, eprintln, format, print, println, vec};
    }
    pub mod rust_2018 {
        pub use super::v1::*;
        pub use std::prelude::rust_2018::*;
    }
    pub mod rust_2021 {
        pub use super::v1::*;
        pub use std::prelude::rust_2021::*;
    }
    pub mod rust_2024 {
        pub use super::v1::*;
        pub use std::prelude::rust_2024::*;
    }
}

#[cfg(feature = "std")]
extern crate std;

#[allow(hidden_glob_reexports)]
extern crate alloc;

#[allow(unused_imports)]
#[rustfmt::skip]
mod generated;

#[cfg(not(feature = "std"))]
pub use self::generated::*;

#[cfg(feature = "std")]
pub use std::*;
