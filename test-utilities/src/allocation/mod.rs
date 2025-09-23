use address::PhysAddr;
use allocation_abstractions::IFrameAllocator;

pub mod contiguous;
pub mod segment;

pub trait ITestFrameAllocator: IFrameAllocator {
    fn check_paddr(&self, paddr: PhysAddr, len: usize) -> bool;

    fn linear_map(&self, paddr: PhysAddr) -> Option<*mut u8>;
}
