#![feature(cfg_accessible)]
#![feature(allocator_api)]
#![cfg_attr(not(feature = "std"), no_std)]

use address::{PhysAddr, PhysPage, PhysPageRange};
use alloc::vec::Vec;
use allocation_abstractions::{FrameDesc, FrameRangeDesc, IFrameAllocator};

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub struct FrameAllocator {
    top: PhysAddr,
    bottom: PhysAddr,
    // current should always point to the last frame that can be allocated
    current: PhysAddr,
    recycled: Vec<PhysAddr>,
}

impl FrameAllocator {
    pub fn new(top: PhysAddr, bottom: PhysAddr) -> Self {
        FrameAllocator {
            top,
            bottom,
            current: bottom,
            recycled: Vec::new(),
        }
    }

    pub fn top(&self) -> PhysAddr {
        self.top
    }

    pub fn bottom(&self) -> PhysAddr {
        self.bottom
    }
}

impl IFrameAllocator for FrameAllocator {
    fn alloc_frame(&mut self) -> Option<FrameDesc> {
        match self.recycled.pop() {
            Some(pa) => Some(unsafe { FrameDesc::new(pa) }),
            None => match self.current {
                pa if pa < self.top => {
                    self.current = pa + constants::PAGE_SIZE;
                    Some(unsafe { FrameDesc::new(pa) })
                }
                _ => None,
            },
        }
    }

    fn alloc_frames(&mut self, count: usize) -> Option<Vec<FrameDesc>> {
        let mut frames = Vec::with_capacity(count);

        let avaliable = self.recycled.len() + (*self.top - *self.bottom) / (constants::PAGE_SIZE);

        match count {
            count if count <= avaliable => {
                for _ in 0..count {
                    match self.alloc_frame() {
                        Some(frame) => frames.push(frame),
                        None => break,
                    }
                }
                Some(frames)
            }
            // Prevent dealloc if we don't have enough frames
            _ => None,
        }
    }

    fn dealloc(&mut self, frame: FrameDesc) {
        // is valid frame
        debug_assert!(frame.0 >= self.bottom && frame.0 < self.top);
        // is allocated frame
        debug_assert!(self.recycled.iter().all(|ppn| *ppn != frame.0) && self.current != frame.0);

        let pa = frame.0;
        core::mem::forget(frame);

        debug_assert!(pa < self.current);

        self.recycled.push(pa);
        self.recycled.sort();

        // try gc self.current before push to recycled
        // Check if the recycled or ppn can be contiguous
        match self.recycled.last() {
            Some(last) if *last + 1usize == self.current => {
                let mut new_current = self.current;

                loop {
                    match self.recycled.pop() {
                        Some(pa) if pa + constants::PAGE_SIZE == new_current => {
                            new_current = pa;
                        }
                        Some(pa) => {
                            self.recycled.push(pa);
                            break;
                        }
                        None => break,
                    }
                }

                self.current = new_current;
            }
            _ => (),
        }
    }

    fn alloc_contiguous(&mut self, count: usize) -> Option<FrameRangeDesc> {
        let avaliable = *self.top - *self.current;

        match count {
            count if count < avaliable => {
                let range = PhysPageRange::new(PhysPage::new_4k(self.current).unwrap(), count);

                self.current += range.as_addr_range().len();

                Some(unsafe { FrameRangeDesc::new(range) })
            }
            // Prevent dealloc if we don't have enough frames
            _ => None,
        }
    }

    fn dealloc_range(&mut self, range: FrameRangeDesc) {
        for page in range.iter() {
            debug_assert!(page.size() == constants::PAGE_SIZE);

            self.dealloc(unsafe { FrameDesc::new(page.addr()) });
        }

        core::mem::forget(range);
    }
}
