use std::{alloc::GlobalAlloc, println};

#[global_allocator]
static DUMMY_ALLOCATOR: DummyAllocator = DummyAllocator;

struct DummyAllocator;

unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        println!("Allocating {} bytes", layout.size());
        core::ptr::null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, layout: core::alloc::Layout) {
        println!("Deallocating {} bytes", layout.size());
    }
}
