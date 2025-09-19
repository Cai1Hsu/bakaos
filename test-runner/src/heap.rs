use std::symbol_ptr;

use buddy_system_allocator::LockedHeap;

#[global_allocator]
static ALLOC: LockedHeap<32> = LockedHeap::new();

pub fn init() {
    let heap_start = unsafe { symbol_ptr!("__heap_start").cast::<u8>() };
    let heap_end = unsafe { symbol_ptr!("__heap_end").cast::<u8>() };
    let size = unsafe { heap_end.offset_from(heap_start) as usize };

    unsafe {
        ALLOC.lock().init(heap_start.as_ptr() as usize, size);
    }
}
