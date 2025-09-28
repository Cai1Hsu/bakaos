#![cfg_attr(not(feature = "std"), no_std)]

use address::PhysAddrRange;
use alloc::vec::Vec;

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod frame;

pub use frame::*;

pub trait IFrameAllocator {
    fn alloc_frame(&mut self) -> Option<FrameDesc>;
    // Allocates `count` frames and returns them as a vector, no guarantee that the frames are contiguous
    fn alloc_frames(&mut self, count: usize) -> Option<Vec<FrameDesc>>;
    // Allocates `count` frames and returns them as a range, guaranteeing that the frames are contiguous
    fn alloc_contiguous(&mut self, count: usize) -> Option<FrameRangeDesc>;

    fn dealloc(&mut self, frame: FrameDesc);

    fn dealloc_range(&mut self, range: FrameRangeDesc);

    fn check_paddr(&self, paddr: PhysAddrRange) -> bool;

    /// Try to get a slice of the physical address in the linear mapping window.
    ///
    /// The slice is guaranteed to be contiguous.
    ///
    /// # Parameters
    ///
    /// - `paddr`: The physical address range you want to access.
    ///
    /// # Returns
    ///
    /// - The raw memory slice to the physical address range in the linear mapping window.
    /// - The return value is `None` if the physical address is not in the linear mapping window.
    ///   Or if the frame allocator is not responsible for the linear mapping. This method is
    ///   intended to provide a backend option for MMU.
    ///
    /// # Safety
    ///
    /// The returned slice is static, caller must ensure that the slice is not used after the memory is deallocated.
    /// This can be done by ensuring that the frame allocator outlives all usage of the slice.
    #[allow(clippy::mut_from_ref)]
    unsafe fn linear_map(&self, paddr: PhysAddrRange) -> Option<&'static mut [u8]>;
}
