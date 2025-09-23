use core::{alloc::Layout, ptr::NonNull};
use std::{collections::BTreeMap, sync::Arc};

use address::{PhysAddr, PhysAddrRange, PhysPage, PhysPageRange};
use allocation_abstractions::{FrameDesc, FrameRangeDesc, IFrameAllocator};
use hermit_sync::SpinMutex;
use mmu_abstractions::IMMU;

use crate::memory::TestMMU;

pub struct TestFrameAllocator {
    records: BTreeMap<PhysAddr, HostMemory>,
}

unsafe impl Send for TestFrameAllocator {}
unsafe impl Sync for TestFrameAllocator {}

impl TestFrameAllocator {
    pub fn new() -> Arc<SpinMutex<TestFrameAllocator>> {
        Arc::new(SpinMutex::new(TestFrameAllocator {
            records: BTreeMap::new(),
        }))
    }

    #[allow(clippy::type_complexity)]
    pub fn new_with_mmu() -> (
        Arc<SpinMutex<dyn IFrameAllocator>>,
        Arc<SpinMutex<dyn IMMU>>,
    ) {
        let alloc = Arc::new(SpinMutex::new(TestFrameAllocator {
            records: BTreeMap::new(),
        }));

        (alloc.clone(), TestMMU::new(alloc))
    }
}

pub(crate) struct HostMemory {
    pub ptr: NonNull<u8>,
    pub layout: Layout,
}

impl HostMemory {
    pub fn alloc(num_frames: usize) -> (PhysAddr, Self) {
        let layout = create_layout(num_frames);
        let (pa, ptr) = heap_allocate(layout);

        (pa, Self { ptr, layout })
    }

    pub fn paddr(&self) -> PhysAddr {
        PhysAddr::new(self.ptr.as_ptr() as usize)
    }

    pub fn paddr_range(&self) -> PhysAddrRange {
        PhysAddrRange::from_start_len(self.paddr(), self.layout.size())
    }
}

impl Drop for HostMemory {
    fn drop(&mut self) {
        heap_deallocate(self.ptr, self.layout);
    }
}

impl IFrameAllocator for TestFrameAllocator {
    fn alloc_frame(&mut self) -> Option<allocation_abstractions::FrameDesc> {
        let (pa, mem) = HostMemory::alloc(1);

        self.records.insert(pa, mem);

        Some(unsafe { FrameDesc::new(pa) })
    }

    fn alloc_frames(&mut self, count: usize) -> Option<Vec<allocation_abstractions::FrameDesc>> {
        let mut v = Vec::with_capacity(count);

        for _ in 0..count {
            v.push(self.alloc_frame()?);
        }

        Some(v)
    }

    fn alloc_contiguous(
        &mut self,
        count: usize,
    ) -> Option<allocation_abstractions::FrameRangeDesc> {
        let (pa, mem) = HostMemory::alloc(count);

        self.records.insert(pa, mem);

        Some(unsafe {
            FrameRangeDesc::new(PhysPageRange::new(PhysPage::new_4k(pa).unwrap(), count))
        })
    }

    fn dealloc(&mut self, frame: allocation_abstractions::FrameDesc) {
        self.records.remove(&frame.0);
        core::mem::forget(frame);
    }

    fn dealloc_range(&mut self, range: allocation_abstractions::FrameRangeDesc) {
        self.records.remove(&range.start().addr());
        core::mem::forget(range);
    }

    fn check_paddr(&self, paddr: PhysAddrRange) -> bool {
        for mem in self.records.values() {
            let target_range = mem.paddr_range();

            if target_range.contains(paddr) {
                return true;
            }
        }

        false
    }

    unsafe fn linear_map(&self, paddr: PhysAddrRange) -> Option<&'static mut [u8]> {
        if self.check_paddr(paddr) {
            Some(std::slice::from_raw_parts_mut(
                *paddr.start() as *mut u8,
                paddr.len(),
            ))
        } else {
            None
        }
    }
}

const fn create_layout(num_frame: usize) -> Layout {
    unsafe {
        Layout::from_size_align_unchecked(constants::PAGE_SIZE * num_frame, constants::PAGE_SIZE)
    }
}

fn heap_allocate(layout: Layout) -> (PhysAddr, NonNull<u8>) {
    let raw_ptr = unsafe { std::alloc::alloc_zeroed(layout) };

    (PhysAddr::new(raw_ptr as usize), unsafe {
        NonNull::new_unchecked(raw_ptr)
    })
}

fn heap_deallocate(ptr: NonNull<u8>, layout: Layout) {
    unsafe { std::alloc::dealloc(ptr.as_ptr(), layout) }
}
