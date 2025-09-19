#![no_std]
#![cfg_attr(target_os = "none", no_main)]
extern crate runtime as std;

#[std::rust_main]
fn main() {
    std::println!("Hello, world!");
}

#[cfg(target_os = "none")]
mod alloc {
    use std::{alloc::GlobalAlloc, println};

    #[global_allocator]
    static DUMMY_ALLOCATOR: DummyAllocator = DummyAllocator;

    struct DummyAllocator;

    unsafe impl GlobalAlloc for DummyAllocator {
        unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
            println!("Allocating {} bytes", layout.size());
            core::ptr::null_mut()
        }
    
        unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
            println!("Deallocating {} bytes", layout.size());
        }
    }
}
