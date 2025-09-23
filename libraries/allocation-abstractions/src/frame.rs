use core::ops::{Deref, Drop};

use address::{PhysAddr, PhysPageRange};

#[derive(Debug)]
pub struct FrameDesc(pub PhysAddr);

impl FrameDesc {
    /// Create a new frame descriptor
    ///
    /// # Safety
    ///
    /// The caller must ensure that the frame is allocated.
    ///
    /// The caller is responsible for deallocating the frame.
    pub unsafe fn new(addr: PhysAddr) -> Self {
        FrameDesc(addr)
    }
}

impl Deref for FrameDesc {
    type Target = PhysAddr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for FrameDesc {
    fn drop(&mut self) {
        panic!("You must manually deallocate frames")
    }
}

pub struct FrameRangeDesc {
    range: PhysPageRange,
}

impl FrameRangeDesc {
    /// Create a new frame range descriptor
    ///
    /// # Safety
    ///
    /// The caller must ensure that the frames are allocated.
    ///
    /// The caller is responsible for deallocating the frames.
    pub unsafe fn new(range: PhysPageRange) -> Self {
        Self { range }
    }
}

impl Deref for FrameRangeDesc {
    type Target = PhysPageRange;

    fn deref(&self) -> &Self::Target {
        &self.range
    }
}

impl Drop for FrameRangeDesc {
    fn drop(&mut self) {
        panic!("You must manually deallocate frames")
    }
}
