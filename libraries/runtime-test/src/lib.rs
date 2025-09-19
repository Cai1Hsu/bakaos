#![no_std]

#[derive(Debug)]
pub enum ResultExpectation {
    Success,
    ShouldPanic,
    ShouldPanicWithMessage(&'static str),
}

#[repr(C)]
#[derive(Debug)]
pub struct SourcePosition {
    pub line: usize,
    pub column: usize,
}

#[repr(C)]
#[derive(Debug)]
pub struct TestDesc {
    pub name: &'static str,
    pub module_path: &'static str,
    pub package: &'static str,
    pub source_file: &'static str,
    pub expect: ResultExpectation,
    pub start: SourcePosition,
    pub end: SourcePosition,
    pub func: fn() -> (),
}
