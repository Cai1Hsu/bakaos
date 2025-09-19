#![no_std]
#![cfg_attr(target_os = "none", no_main)]
extern crate runtime as std;

// see https://github.com/rust-lang/rust/issues/133491#issue-2694064193
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

mod heap;

use std::{
    println, rust_main,
    test::{ResultExpectation, TestDesc},
};

#[rust_main]
pub fn main() {
    println!("Hello, world!");

    let tests = collect_tests();

    // very basic test runner

    println!("Collecting {} tests", tests.len());
    for test in tests {
        println!(" - {} (expect: {:?})", test.name, test.expect);
    }

    println!("Running tests...");
    for test in tests {
        // panic unwind not supported yet
        if !matches!(test.expect, ResultExpectation::Success) {
            println!("test {} ... ignored", test.name);
            continue;
        }
        (test.func)();
        println!("test {} ... ok", test.name);
    }
}

fn collect_tests() -> &'static [TestDesc] {
    #[cfg(not(target_os = "none"))]
    {
        // Since direct `cargo test` is supported on hosted platform,
        // We didn't implement a way to collect tests on hosted platform yet.
        &[]
    }

    #[cfg(target_os = "none")]
    {
        use std::{slice, symbol_ptr};

        unsafe {
            let start = symbol_ptr!("__sktest_array").cast::<TestDesc>();
            let end = symbol_ptr!("__ektest_array").cast::<TestDesc>();

            slice::from_raw_parts(start.as_ptr(), end.offset_from(start) as usize)
        }
    }
}
