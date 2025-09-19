#![no_std]
#![coverage(off)]
#![feature(coverage_attribute)]
#![cfg_attr(target_os = "none", no_main)]
extern crate runtime as std;

// see https://github.com/rust-lang/rust/issues/133491#issue-2694064193
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[cfg(target_os = "none")]
mod heap;

use std::{
    fmt::Display,
    println, rust_main,
    string::{String, ToString},
    test::{ResultExpectation, TestDesc},
};

#[rust_main]
pub fn main() {
    #[cfg(target_os = "none")]
    heap::init();

    let tests = collect_tests();

    // very basic test runner

    println!("Collecting {} tests", tests.len());
    for test in tests {
        println!(" - {} (expect: {:?})", test.name, test.expect);
    }

    println!("Running tests...");

    let mut passed = 0;
    let mut failed = 0;
    for test in tests {
        let run_result = run_single_test(test);
        let test_result = TestResult::new(test, run_result);

        if test_result.is_passed() {
            passed += 1;
        } else {
            failed += 1;
        }

        println!("test {} ... {}", test.name, test_result);
    }

    println!("test result: {} passed; {} failed", passed, failed);
}

enum TestResult {
    UnexpectedPanic(PanicPayload),
    MismatchedPanic(PanicPayload, String /* expected */),
    ExpectedPanicWithMessage(PanicPayload, String /* expected */),
    MissingPanic,
    ExpectedPanic(PanicPayload),
    Ok,
}

impl TestResult {
    fn new(test: &TestDesc, run_result: RunResult) -> Self {
        match (&test.expect, run_result) {
            (ResultExpectation::Success, RunResult::ExitedNormally) => TestResult::Ok,
            (ResultExpectation::Success, RunResult::Panicked(payload)) => {
                TestResult::UnexpectedPanic(payload)
            }
            (ResultExpectation::ShouldPanicWithMessage(expected), RunResult::Panicked(payload))
                if payload.message.contains(expected) =>
            {
                TestResult::ExpectedPanicWithMessage(payload, expected.to_string())
            }
            (ResultExpectation::ShouldPanicWithMessage(expected), RunResult::Panicked(payload)) => {
                TestResult::MismatchedPanic(payload, expected.to_string())
            }
            (ResultExpectation::ShouldPanic, RunResult::Panicked(payload)) => {
                TestResult::ExpectedPanic(payload)
            }
            (
                ResultExpectation::ShouldPanic | ResultExpectation::ShouldPanicWithMessage(_),
                RunResult::ExitedNormally,
            ) => TestResult::MissingPanic,
        }
    }

    fn is_passed(&self) -> bool {
        matches!(self, TestResult::Ok | TestResult::ExpectedPanic(_))
    }
}

impl Display for TestResult {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TestResult::Ok => write!(f, "ok"),
            TestResult::UnexpectedPanic(payload) => write!(
                f,
                "failed: unexpected panic: {} at {}:{}:{}",
                payload.message, payload.file, payload.line, payload.col
            ),
            TestResult::MismatchedPanic(payload, expected) => write!(
                f,
                "failed: panic message mismatch: got '{}', expected to contain '{}'",
                payload.message, expected
            ),
            TestResult::ExpectedPanicWithMessage(payload, expected) => {
                write!(
                    f,
                    "ok (expected panic with message containing '{}'). Full message: '{}'",
                    expected, payload.message
                )
            }
            TestResult::MissingPanic => {
                write!(f, "failed: expected panic but test exited normally")
            }
            TestResult::ExpectedPanic(payload) => write!(
                f,
                "ok (expected panic). Full message: '{}'",
                payload.message
            ),
        }
    }
}

struct PanicPayload {
    pub message: String,
    pub file: String,
    pub line: usize,
    pub col: usize,
}

enum RunResult {
    ExitedNormally,
    Panicked(PanicPayload),
}

fn run_single_test(test: &TestDesc) -> RunResult {
    #[cfg(not(target_os = "none"))]
    use std::panic::catch_unwind;
    #[cfg(target_os = "none")]
    use unwinding::panic::catch_unwind;

    let ret = catch_unwind(|| {
        (test.func)();
    });

    match ret {
        Ok(()) => RunResult::ExitedNormally,
        Err(payload) => RunResult::Panicked(*payload.downcast().unwrap()),
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

#[cfg(target_os = "none")]
mod panicking {
    use super::*;
    use std::{
        boxed::Box,
        panic::{Location, PanicInfo},
    };

    #[unsafe(no_mangle)]
    unsafe extern "Rust" fn __panic_handler_impl(info: &PanicInfo) -> ! {
        let location = info.location().unwrap_or(&Location::caller());
        let payload = PanicPayload {
            message: info.message().to_string(),
            file: location.file().to_string(),
            line: location.line() as usize,
            col: location.column() as usize,
        };

        let reason = unwinding::panic::begin_panic(Box::new(payload));

        println!(
            "Uncaught panic at {}:{}:{}, reason: {}\n{}",
            location.file(),
            location.line(),
            location.column(),
            reason.0,
            info.message(),
        );

        loop {}
    }
}
