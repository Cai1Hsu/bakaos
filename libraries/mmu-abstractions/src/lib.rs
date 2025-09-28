#![cfg_attr(not(feature = "std"), no_std)]

use core::ops::{Deref, DerefMut};

use address::{PhysAddr, VirtAddr};

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod flags;

use alloc::sync::Arc;
use allocation_abstractions::IFrameAllocator;
use downcast_rs::{impl_downcast, Downcast};
pub use flags::GenericMappingFlags;
use hermit_sync::SpinMutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MMUError {
    InvalidAddress,
    PrivilegeError,
    AccessFault, // not mapped to a proper frame
    MisalignedAddress,
    Borrowed,
    CanNotModify,
    PageNotReadable { vaddr: VirtAddr },
    PageNotWritable { vaddr: VirtAddr },
}

impl dyn IMMU {
    pub fn inspect_framed(
        &self,
        vaddr: VirtAddr,
        len: usize,
        mut callback: impl FnMut(&[u8], usize) -> bool,
    ) -> Result<(), MMUError> {
        self.inspect_framed_internal(vaddr, len, &mut callback)
    }

    pub fn inspect_framed_mut(
        &self,
        vaddr: VirtAddr,
        len: usize,
        mut callback: impl FnMut(&mut [u8], usize) -> bool,
    ) -> Result<(), MMUError> {
        self.inspect_framed_mut_internal(vaddr, len, &mut callback)
    }

    pub fn import<T: Copy>(&self, vaddr: VirtAddr) -> Result<T, MMUError> {
        let mut value: T = unsafe { core::mem::zeroed() };
        let value_bytes = unsafe {
            core::slice::from_raw_parts_mut(
                &mut value as *mut T as *mut u8,
                core::mem::size_of::<T>(),
            )
        };

        self.read_bytes(vaddr, value_bytes).map(|_| value)
    }

    pub fn export<T: Copy>(&self, vaddr: VirtAddr, value: T) -> Result<(), MMUError> {
        let value_bytes = unsafe {
            core::slice::from_raw_parts(&value as *const T as *const u8, core::mem::size_of::<T>())
        };

        self.write_bytes(vaddr, value_bytes)
    }

    pub fn map_buffer(&self, vaddr: VirtAddr, len: usize) -> Result<Memory<'_>, MMUError> {
        #[allow(deprecated)]
        self.map_buffer_internal(vaddr, len).map(|buf| Memory {
            mmu: self,
            slice: buf,
        })
    }

    pub fn map_buffer_mut(
        &self,
        vaddr: VirtAddr,
        len: usize,
        force_mut: bool,
    ) -> Result<MemoryMut<'_>, MMUError> {
        #[allow(deprecated)]
        self.map_buffer_mut_internal(vaddr, len, force_mut)
            .map(|buf| MemoryMut {
                mmu: self,
                slice: buf,
            })
    }

    #[cfg(not(target_os = "none"))]
    pub fn register<T>(&mut self, val: &T, mutable: bool) -> VirtAddr {
        let vaddr = VirtAddr::from(val);

        self.register_internal(vaddr, core::mem::size_of_val(val), mutable);

        vaddr
    }

    #[cfg(not(target_os = "none"))]
    pub fn unregister<T>(&mut self, val: &T) {
        self.unregister_internal(VirtAddr::from(val));
    }
}

pub trait IMMU: Downcast {
    fn bound_alloc(&self) -> Option<Arc<SpinMutex<dyn IFrameAllocator>>>;

    fn map_single(
        &mut self,
        vaddr: VirtAddr,
        target: PhysAddr,
        size: PageSize,
        flags: GenericMappingFlags,
    ) -> PagingResult<()>;

    fn remap_single(
        &mut self,
        vaddr: VirtAddr,
        new_target: PhysAddr,
        flags: GenericMappingFlags,
    ) -> PagingResult<PageSize>;

    fn unmap_single(&mut self, vaddr: VirtAddr) -> PagingResult<(PhysAddr, PageSize)>;

    fn query_virtual(
        &self,
        vaddr: VirtAddr,
    ) -> PagingResult<(PhysAddr, GenericMappingFlags, PageSize)>;

    fn create_or_update_single(
        &mut self,
        vaddr: VirtAddr,
        size: PageSize,
        paddr: Option<PhysAddr>,
        flags: Option<GenericMappingFlags>,
    ) -> PagingResult<()>;

    #[doc(hidden)]
    fn inspect_framed_internal(
        &self,
        vaddr: VirtAddr,
        len: usize,
        callback: &mut dyn FnMut(&[u8], usize) -> bool,
    ) -> Result<(), MMUError>;

    #[doc(hidden)]
    fn inspect_framed_mut_internal(
        &self,
        vaddr: VirtAddr,
        len: usize,
        callback: &mut dyn FnMut(&mut [u8], usize) -> bool,
    ) -> Result<(), MMUError>;

    fn linear_map_phys(&self, paddr: PhysAddr, len: usize) -> Result<&'static mut [u8], MMUError>;

    fn read_bytes(&self, vaddr: VirtAddr, buf: &mut [u8]) -> Result<(), MMUError>;

    fn write_bytes(&self, vaddr: VirtAddr, buf: &[u8]) -> Result<(), MMUError>;

    /// Maps a memory area from another MMU.
    fn map_cross_internal<'a>(
        &'a mut self,
        source: &'a dyn IMMU,
        vaddr: VirtAddr,
        len: usize,
    ) -> Result<&'a [u8], MMUError>;

    /// Maps a mutable memory area from another MMU.
    fn map_cross_mut_internal<'a>(
        &'a mut self,
        source: &'a dyn IMMU,
        vaddr: VirtAddr,
        len: usize,
    ) -> Result<&'a mut [u8], MMUError>;

    #[doc(hidden)]
    #[deprecated = "Do not use this method, use `map_buffer` from dyn IMMU"]
    fn map_buffer_internal(&self, vaddr: VirtAddr, len: usize) -> Result<&'_ [u8], MMUError>;

    /// Get a mutable reference to the given memory area.
    /// The returned slice may not points to vaddr.
    #[doc(hidden)]
    #[deprecated = "Do not use this method, use `map_buffer_mut` from dyn IMMU"]
    #[allow(clippy::mut_from_ref)]
    fn map_buffer_mut_internal(
        &self,
        vaddr: VirtAddr,
        len: usize,
        force_mut: bool,
    ) -> Result<&'_ mut [u8], MMUError>;

    fn unmap_buffer(&self, vaddr: VirtAddr);

    fn unmap_cross(&mut self, source: &dyn IMMU, vaddr: VirtAddr);

    fn platform_payload(&self) -> usize;

    #[doc(hidden)]
    #[cfg(not(target_os = "none"))]
    fn register_internal(&mut self, vaddr: VirtAddr, len: usize, mutable: bool);

    #[doc(hidden)]
    #[cfg(not(target_os = "none"))]
    fn unregister_internal(&mut self, vaddr: VirtAddr);
}

impl_downcast!(IMMU);

/// The error type for page table operation failures.
#[derive(Debug, PartialEq, Eq)]
pub enum PagingError {
    /// The address is not aligned to the page size.
    NotAligned,
    /// The mapping is not present.
    NotMapped,
    /// The mapping is already present.
    AlreadyMapped,
    /// The page table entry represents a huge page, but the target physical
    /// frame is 4K in size.
    MappedToHugePage,
    CanNotModify,
    OutOfMemory,
}

// cargo clippy keeps complaining about use <XXXX as Into<T>>::into(e) if From is implemented
// so we use Into over From
#[allow(clippy::from_over_into)]
impl Into<MMUError> for PagingError {
    fn into(self) -> MMUError {
        match self {
            PagingError::NotAligned => MMUError::MisalignedAddress,
            PagingError::NotMapped => MMUError::InvalidAddress,
            _ => unimplemented!("Should never happen: {:?}", self),
        }
    }
}

/// The page sizes supported by the hardware page table.
#[repr(usize)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PageSize {
    /// Size of 4 kilobytes (2<sup>12</sup> bytes).
    _4K = 0x1000,
    /// Size of 2 megabytes (2<sup>21</sup> bytes).
    _2M = 0x20_0000,
    /// Size of 1 gigabytes (2<sup>30</sup> bytes).
    _1G = 0x4000_0000,
    Custom(usize),
}

impl From<usize> for PageSize {
    fn from(value: usize) -> Self {
        match value {
            0x1000 => PageSize::_4K,
            0x20_0000 => PageSize::_2M,
            0x4000_0000 => PageSize::_1G,
            _ => PageSize::Custom(value),
        }
    }
}

impl PageSize {
    pub const fn as_usize(&self) -> usize {
        match self {
            PageSize::_4K => 0x1000,
            PageSize::_2M => 0x20_0000,
            PageSize::_1G => 0x4000_0000,
            PageSize::Custom(v) => *v,
        }
    }
}

pub type PagingResult<TValue> = Result<TValue, PagingError>;

pub struct Memory<'a> {
    mmu: &'a dyn IMMU,
    slice: &'a [u8],
}

impl Deref for Memory<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.slice
    }
}

impl Drop for Memory<'_> {
    fn drop(&mut self) {
        self.mmu.unmap_buffer(VirtAddr::from(self.slice.as_ptr()));
    }
}

pub struct MemoryMut<'a> {
    mmu: &'a dyn IMMU,
    slice: &'a mut [u8],
}

impl Deref for MemoryMut<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.slice
    }
}

impl DerefMut for MemoryMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.slice
    }
}

impl Drop for MemoryMut<'_> {
    fn drop(&mut self) {
        self.mmu.unmap_buffer(VirtAddr::from(self.slice.as_ptr()));
    }
}
