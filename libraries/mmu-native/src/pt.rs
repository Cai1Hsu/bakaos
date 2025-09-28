use core::{marker::PhantomData, ops::Deref};
use runtime::baremetal::vm::get_linear_vaddr;

use crate::IArchPageTableEntry;
use address::{PhysAddr, PhysPage, VirtAddr, VirtAddrRange, VirtPage};
use alloc::{collections::btree_set::BTreeSet, sync::Arc, vec, vec::Vec};
use allocation_abstractions::{FrameDesc, IFrameAllocator};
use hermit_sync::SpinMutex;
use mmu_abstractions::{GenericMappingFlags, MMUError, PageSize, PagingError, PagingResult, IMMU};
use utilities::InvokeOnDrop;

pub trait IPageTableArchAttribute {
    const LEVELS: usize;
    const PA_MAX_BITS: usize;
    const VA_MAX_BITS: usize;
    const PA_MAX_ADDR: usize = (1 << Self::PA_MAX_BITS) - 1;
}

pub struct PageTableNative<Arch, PTE>
where
    Arch: IPageTableArchAttribute,
    PTE: IArchPageTableEntry,
{
    root: PhysAddr,
    allocation: Option<PageTableAllocation>,
    _marker: PhantomData<(Arch, PTE)>,
}

unsafe impl<A: IPageTableArchAttribute, P: IArchPageTableEntry> Send for PageTableNative<A, P> {}
unsafe impl<A: IPageTableArchAttribute, P: IArchPageTableEntry> Sync for PageTableNative<A, P> {}

struct PageTableAllocation {
    frames: Vec<FrameDesc>,
    allocator: Arc<SpinMutex<dyn IFrameAllocator>>,
    cross_mappings: SpinMutex<CrossMappingAllocator>,
}

struct CrossMappingAllocator {
    base: VirtAddr,
    windows: BTreeSet<CrossMappingWindow>,
}

impl CrossMappingAllocator {
    pub fn new(base: VirtAddr) -> Self {
        Self {
            base,
            windows: BTreeSet::new(),
        }
    }

    pub fn alloc(&mut self, size: usize, mutable: bool) -> VirtAddr {
        let vaddr = self
            .windows
            .last()
            .map(|window| window.vaddr + window.size)
            .unwrap_or(self.base);

        let window = CrossMappingWindow {
            vaddr,
            size,
            mutable,
        };

        self.windows.insert(window);

        vaddr
    }

    pub fn remove(&mut self, vaddr: VirtAddr) -> Option<CrossMappingWindow> {
        let mut target = None;
        for w in self.windows.iter() {
            if w.vaddr_range().contains_addr(vaddr) {
                target = Some(w.clone());
                break;
            }
        }

        if let Some(target) = target.as_ref() {
            self.windows.remove(target);
        }

        target
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CrossMappingWindow {
    vaddr: VirtAddr,
    size: usize,
    mutable: bool,
}

impl CrossMappingWindow {
    pub fn vaddr_range(&self) -> VirtAddrRange {
        VirtAddrRange::from_start_len(self.vaddr, self.size)
    }
}

impl PartialOrd for CrossMappingWindow {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CrossMappingWindow {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.vaddr.cmp(&other.vaddr)
    }
}

impl Drop for PageTableAllocation {
    fn drop(&mut self) {
        while let Some(frame) = self.frames.pop() {
            self.allocator.lock().dealloc(frame);
        }
    }
}

impl<Arch: IPageTableArchAttribute + 'static, PTE: IArchPageTableEntry + 'static> IMMU
    for PageTableNative<Arch, PTE>
{
    fn map_single(
        &mut self,
        vaddr: VirtAddr,
        target: PhysAddr,
        size: PageSize,
        flags: GenericMappingFlags,
    ) -> PagingResult<()> {
        if let Some(target_page) = PhysPage::new_custom(target, size.as_usize()) {
            let entry = self.get_create_entry(vaddr, size)?;
            if !entry.is_empty() {
                return Err(PagingError::AlreadyMapped);
            }

            *entry = PTE::new_page(target_page.addr(), flags, size != PageSize::_4K);

            Ok(())
        } else {
            Err(PagingError::NotAligned)
        }
    }

    fn remap_single(
        &mut self,
        vaddr: VirtAddr,
        new_target: PhysAddr,
        flags: GenericMappingFlags,
    ) -> PagingResult<PageSize> {
        if PhysPage::new_4k(new_target).is_none() {
            return Err(PagingError::NotAligned);
        }

        let (entry, size) = self.get_entry_mut(vaddr)?;

        if let Some(target_page) = PhysPage::new_custom(new_target, size.as_usize()) {
            entry.set_paddr(target_page.addr());
            entry.set_flags(flags, size != PageSize::_4K);
            Ok(size)
        } else {
            Err(PagingError::NotAligned)
        }
    }

    fn unmap_single(&mut self, vaddr: VirtAddr) -> PagingResult<(PhysAddr, PageSize)> {
        let (entry, size) = self.get_entry_mut(vaddr)?;
        if !entry.is_present() {
            entry.clear();
            return Err(PagingError::NotMapped);
        }

        let paddr = entry.paddr();

        entry.clear();

        Ok((paddr, size))
    }

    fn query_virtual(
        &self,
        vaddr: VirtAddr,
    ) -> PagingResult<(PhysAddr, GenericMappingFlags, PageSize)> {
        let (entry, size) = self.get_entry(vaddr.align_down(constants::PAGE_SIZE))?;

        if entry.is_empty() {
            return Err(PagingError::NotMapped);
        }

        let offset = vaddr.offset_from_alignment(size.as_usize());

        Ok((entry.paddr() | offset, entry.flags(), size))
    }

    fn create_or_update_single(
        &mut self,
        vaddr: VirtAddr,
        size: PageSize,
        paddr: Option<PhysAddr>,
        flags: Option<GenericMappingFlags>,
    ) -> PagingResult<()> {
        let entry = self.get_create_entry(vaddr, size)?;

        if let Some(paddr) = paddr {
            entry.set_paddr(paddr);
        }

        if let Some(flags) = flags {
            entry.set_flags(flags, size != PageSize::_4K);
        }

        Ok(())
    }

    fn platform_payload(&self) -> usize {
        *self.root
    }

    fn read_bytes(&self, vaddr: VirtAddr, buf: &mut [u8]) -> Result<(), MMUError> {
        let mut bytes_read = 0;
        self.inspect_bytes_through_linear(vaddr, buf.len(), |src| {
            buf[bytes_read..bytes_read + src.len()].copy_from_slice(src);

            bytes_read += src.len();
            true
        })
    }

    fn write_bytes(&self, vaddr: VirtAddr, buf: &[u8]) -> Result<(), MMUError> {
        let mut bytes_written = 0;
        self.inspect_bytes_through_linear(vaddr, buf.len(), |dst| {
            dst.copy_from_slice(&buf[bytes_written..bytes_written + dst.len()]);

            bytes_written += dst.len();
            true
        })
    }

    fn inspect_framed_internal(
        &self,
        vaddr: VirtAddr,
        len: usize,
        callback: &mut dyn FnMut(&[u8], usize) -> bool,
    ) -> Result<(), MMUError> {
        ensure_vaddr_valid(vaddr)?;

        let mut checking_vaddr = vaddr;
        let mut remaining_len = len;

        loop {
            let (paddr, flags, size) = self.query_virtual(checking_vaddr).map_err(|e| e.into())?;

            ensure_permission(vaddr, flags, false)?;

            let frame_base = paddr.align_down(size.as_usize());

            let frame_remain_len = size.as_usize() - (paddr - frame_base) as usize;

            let avaliable_len = remaining_len.min(frame_remain_len);

            let slice = unsafe {
                core::slice::from_raw_parts(
                    // query_virtual adds offset internally
                    get_linear_vaddr(*paddr) as *mut u8,
                    avaliable_len,
                )
            };

            if !callback(slice, avaliable_len) {
                break;
            }

            checking_vaddr += frame_remain_len;
            remaining_len -= avaliable_len;

            if remaining_len == 0 {
                break;
            }
        }

        Ok(())
    }

    fn inspect_framed_mut_internal(
        &self,
        vaddr: VirtAddr,
        len: usize,
        callback: &mut dyn FnMut(&mut [u8], usize) -> bool,
    ) -> Result<(), MMUError> {
        ensure_vaddr_valid(vaddr)?;

        let mut checking_vaddr = vaddr;
        let mut remaining_len = len;

        loop {
            let (paddr, flags, size) = self.query_virtual(checking_vaddr).map_err(|e| e.into())?;

            ensure_permission(vaddr, flags, true)?;

            let frame_base = paddr.align_down(size.as_usize());

            let frame_remain_len = size.as_usize() - (paddr - frame_base) as usize;

            let avaliable_len = remaining_len.min(frame_remain_len);

            let slice = unsafe {
                core::slice::from_raw_parts_mut(
                    // query_virtual adds offset internally
                    get_linear_vaddr(*paddr) as *mut u8,
                    avaliable_len,
                )
            };

            if !callback(slice, avaliable_len) {
                break;
            }

            checking_vaddr += frame_remain_len;
            remaining_len -= avaliable_len;

            if remaining_len == 0 {
                break;
            }
        }

        Ok(())
    }

    fn linear_map_phys(&self, paddr: PhysAddr, len: usize) -> Result<&'static mut [u8], MMUError> {
        let virt = get_linear_vaddr(*paddr) as *mut u8;

        Ok(unsafe { core::slice::from_raw_parts_mut(virt, len) })
    }

    fn map_buffer_internal(&self, vaddr: VirtAddr, len: usize) -> Result<&'_ [u8], MMUError> {
        self.inspect_permission(vaddr, len, false)?;

        Ok(unsafe { core::slice::from_raw_parts(vaddr.as_ptr(), len) })
    }

    fn map_buffer_mut_internal(
        &self,
        vaddr: VirtAddr,
        len: usize,
        _force_mut: bool,
    ) -> Result<&'_ mut [u8], MMUError> {
        self.inspect_permission(vaddr, len, true)?;

        Ok(unsafe { core::slice::from_raw_parts_mut(vaddr.as_mut_ptr(), len) })
    }

    fn unmap_buffer(&self, _vaddr: VirtAddr) {}

    fn map_cross_internal<'a>(
        &'a mut self,
        source: &'a dyn IMMU,
        vaddr: VirtAddr,
        len: usize,
    ) -> Result<&'a [u8], MMUError> {
        const PERMISSION: GenericMappingFlags =
            GenericMappingFlags::Readable.union(GenericMappingFlags::Kernel);

        let mut cross = self
            .allocation
            .as_ref()
            .ok_or(MMUError::CanNotModify)?
            .cross_mappings
            .lock();

        let window = cross.alloc(len, false); // placeholder
        let window = InvokeOnDrop::transform(window, |w| {
            cross.remove(w);
        });

        let mut slice_offset = None;

        let end = vaddr + len;
        let mut checking = vaddr;

        let mut phys = Vec::new();

        loop {
            let (phy, permission, sz) = source.query_virtual(checking).map_err(|e| e.into())?;

            ensure_permission(vaddr, permission, false)?;
            phys.push((phy, sz));

            let page_offset = vaddr.offset_from_alignment(sz.as_usize());
            if slice_offset.is_none() {
                slice_offset = Some(page_offset);
            }

            let sz = sz.as_usize();

            // align to page size
            checking -= page_offset;

            if checking + sz >= end {
                let vaddr = *window.deref();
                window.cancel(); // prevent drop

                drop(cross);

                let mut offset = 0;
                for (phy, sz) in phys {
                    self.map_single(vaddr + offset, phy, sz, PERMISSION)
                        .unwrap();
                    offset += sz.as_usize();
                }

                return Ok(unsafe {
                    core::slice::from_raw_parts(
                        vaddr.as_ptr::<u8>().add(slice_offset.unwrap()),
                        len,
                    )
                });
            }

            checking += sz;
        }
    }

    fn map_cross_mut_internal<'a>(
        &'a mut self,
        source: &'a dyn IMMU,
        vaddr: VirtAddr,
        len: usize,
    ) -> Result<&'a mut [u8], MMUError> {
        const PERMISSION: GenericMappingFlags = GenericMappingFlags::Readable
            .union(GenericMappingFlags::Writable)
            .union(GenericMappingFlags::Kernel);

        let mut cross = self
            .allocation
            .as_ref()
            .ok_or(MMUError::CanNotModify)?
            .cross_mappings
            .lock();

        let window = cross.alloc(len, true); // placeholder
        let window = InvokeOnDrop::transform(window, |w| {
            cross.remove(w);
        });

        let mut slice_offset = None;

        let end = vaddr + len;
        let mut checking = vaddr;

        let mut phys = Vec::new();

        loop {
            let (phy, permission, sz) = source.query_virtual(checking).map_err(|e| e.into())?;

            ensure_permission(vaddr, permission, true)?;
            phys.push((phy, sz));

            let page_offset = vaddr.offset_from_alignment(sz.as_usize());
            if slice_offset.is_none() {
                slice_offset = Some(page_offset);
            }

            let sz = sz.as_usize();

            // align to page size
            checking -= page_offset;

            if checking + sz >= end {
                let vaddr = *window.deref();
                window.cancel(); // prevent drop

                drop(cross);

                let mut offset = 0;
                for (phy, sz) in phys {
                    self.map_single(vaddr + offset, phy, sz, PERMISSION)
                        .unwrap();
                    offset += sz.as_usize();
                }

                return Ok(unsafe {
                    core::slice::from_raw_parts_mut(
                        vaddr.as_mut_ptr::<u8>().add(slice_offset.unwrap()),
                        len,
                    )
                });
            }

            checking += sz;
        }
    }

    fn unmap_cross(&mut self, _source: &dyn IMMU, vaddr: VirtAddr) {
        let mut cross = self.allocation.as_ref().unwrap().cross_mappings.lock();

        if let Some(window) = cross.remove(vaddr) {
            drop(cross);

            self.unmap_single(window.vaddr).ok();
        }
    }

    fn bound_alloc(&self) -> Option<Arc<SpinMutex<dyn IFrameAllocator>>> {
        self.allocation.as_ref().map(|a| a.allocator.clone())
    }
}

impl<Arch: IPageTableArchAttribute + 'static, PTE: IArchPageTableEntry + 'static>
    PageTableNative<Arch, PTE>
{
    fn inspect_permission(
        &self,
        vaddr: VirtAddr,
        len: usize,
        mutable: bool,
    ) -> Result<(), MMUError> {
        ensure_vaddr_valid(vaddr)?;

        let mut checking_vaddr = vaddr;
        let mut remaining_len = len;

        loop {
            let (paddr, flags, size) = self.query_virtual(checking_vaddr).map_err(|e| e.into())?;

            ensure_permission(vaddr, flags, mutable)?;

            let frame_base = paddr.align_down(size.as_usize());

            let frame_remain_len = size.as_usize() - (paddr - frame_base) as usize;
            let avaliable_len = remaining_len.min(frame_remain_len);

            checking_vaddr += frame_remain_len;
            remaining_len -= avaliable_len;

            if remaining_len == 0 {
                break;
            }
        }

        Ok(())
    }

    fn inspect_bytes_through_linear(
        &self,
        vaddr: VirtAddr,
        len: usize,
        mut callback: impl FnMut(&mut [u8]) -> bool,
    ) -> Result<(), MMUError> {
        ensure_vaddr_valid(vaddr)?;

        let mut checking_vaddr = vaddr;
        let mut remaining_len = len;

        loop {
            let (paddr, flags, size) = self.query_virtual(checking_vaddr).map_err(|e| e.into())?;

            ensure_linear_permission(flags)?;

            let frame_base = paddr.align_down(size.as_usize());

            let frame_remain_len = size.as_usize() - (paddr - frame_base) as usize;
            let avaliable_len = remaining_len.min(frame_remain_len);

            {
                let slice = unsafe {
                    core::slice::from_raw_parts_mut(
                        get_linear_vaddr(*paddr) as *mut u8,
                        avaliable_len,
                    )
                };

                if !callback(slice) {
                    return Ok(());
                }
            }

            checking_vaddr += frame_remain_len;
            remaining_len -= avaliable_len;

            if remaining_len == 0 {
                break;
            }
        }

        Ok(())
    }
}

const fn ensure_vaddr_valid(vaddr: VirtAddr) -> Result<(), MMUError> {
    if vaddr.is_null() {
        return Err(MMUError::InvalidAddress);
    }

    Ok(())
}

const fn ensure_linear_permission(flags: GenericMappingFlags) -> Result<(), MMUError> {
    if !flags.contains(GenericMappingFlags::User) {
        return Err(MMUError::PrivilegeError);
    }

    Ok(())
}

const fn ensure_permission(
    vaddr: VirtAddr,
    flags: GenericMappingFlags,
    mutable: bool,
) -> Result<(), MMUError> {
    if !flags.contains(GenericMappingFlags::User) {
        return Err(MMUError::PrivilegeError);
    }

    if !flags.contains(GenericMappingFlags::Readable) {
        return Err(MMUError::PageNotReadable { vaddr });
    }

    if mutable && !flags.contains(GenericMappingFlags::Writable) {
        return Err(MMUError::PageNotWritable { vaddr });
    }

    Ok(())
}

impl<Arch: IPageTableArchAttribute, PTE: IArchPageTableEntry> PageTableNative<Arch, PTE> {
    const fn from_borrowed(root: PhysAddr) -> Self {
        Self {
            root,
            allocation: None,
            _marker: PhantomData,
        }
    }

    pub fn new(root: PhysAddr, allocator: Option<Arc<SpinMutex<dyn IFrameAllocator>>>) -> Self {
        match allocator {
            None => Self::from_borrowed(root),
            Some(allocator) => Self {
                root,
                allocation: Some(PageTableAllocation {
                    frames: Vec::new(),
                    allocator,
                    cross_mappings: SpinMutex::new(CrossMappingAllocator::new(
                        VirtAddr::null, // FIXME
                    )),
                }),
                _marker: PhantomData,
            },
        }
    }

    pub fn alloc(allocator: Arc<SpinMutex<dyn IFrameAllocator>>) -> Self {
        let frame = allocator.lock().alloc_frame().unwrap();

        let mut pt = Self::from_borrowed(frame.0);

        pt.allocation = Some(PageTableAllocation {
            frames: vec![frame],
            allocator,
            cross_mappings: SpinMutex::new(CrossMappingAllocator::new(
                VirtAddr::null, // FIXME
            )),
        });

        pt
    }

    const fn root(&self) -> PhysAddr {
        self.root
    }

    fn ensure_can_modify(&self) -> PagingResult<&PageTableAllocation> {
        match self.allocation {
            None => Err(PagingError::CanNotModify),
            Some(ref alloc) => Ok(alloc),
        }
    }

    fn ensure_can_modify_mut(&mut self) -> PagingResult<&mut PageTableAllocation> {
        match self.allocation {
            None => Err(PagingError::CanNotModify),
            Some(ref mut alloc) => Ok(alloc),
        }
    }

    /// # Safety
    /// This breaks Rust's mutability rule, use it properly
    #[allow(clippy::mut_from_ref)]
    unsafe fn get_entry_internal(&self, vaddr: VirtAddr) -> PagingResult<(&mut PTE, PageSize)> {
        let vaddr = *vaddr;

        let pt_l3 = if Arch::LEVELS == 3 {
            self.raw_table_of(self.root())?
        } else if Arch::LEVELS == 4 {
            let pt_l4 = self.raw_table_of(self.root())?;
            let pt_l4e = &mut pt_l4[Self::p4_index(vaddr)];
            self.get_next_level(pt_l4e)?
        } else {
            panic!("Unsupported page table");
        };
        let pt_l3e = &mut pt_l3[Self::p3_index(vaddr)];

        if pt_l3e.is_huge() {
            return Ok((pt_l3e, PageSize::_1G));
        }

        let pt_l2 = self.get_next_level(pt_l3e)?;
        let pt_l2e = &mut pt_l2[Self::p2_index(vaddr)];
        if pt_l2e.is_huge() {
            return Ok((pt_l2e, PageSize::_2M));
        }

        let pt_l1 = self.get_next_level(pt_l2e)?;
        let pt_1e = &mut pt_l1[Self::p1_index(vaddr)];
        Ok((pt_1e, PageSize::_4K))
    }

    fn raw_table_of<'a>(&self, paddr: PhysAddr) -> PagingResult<&'a mut [PTE]> {
        if PhysPage::new_4k(paddr).is_none() {
            return Err(PagingError::NotAligned);
        }

        if paddr.is_null() {
            return Err(PagingError::NotMapped);
        }

        let ptr = get_linear_vaddr(*paddr) as *mut _;
        Ok(unsafe { core::slice::from_raw_parts_mut(ptr, Self::NUM_ENTRIES) })
    }

    fn get_next_level<'a>(&self, entry: &PTE) -> PagingResult<&'a mut [PTE]> {
        if !entry.is_present() {
            Err(PagingError::NotMapped)
        } else if entry.is_huge() {
            Err(PagingError::MappedToHugePage)
        } else {
            self.raw_table_of(entry.paddr())
        }
    }

    fn get_entry(&self, vaddr: VirtAddr) -> PagingResult<(&PTE, PageSize)> {
        unsafe {
            self.get_entry_internal(vaddr)
                .map(|(pte, size)| (pte as &_, size))
        }
    }

    fn get_entry_mut(&mut self, vaddr: VirtAddr) -> PagingResult<(&mut PTE, PageSize)> {
        let _ = self.ensure_can_modify()?;

        unsafe { self.get_entry_internal(vaddr) }
    }

    fn get_create_entry(&mut self, vaddr: VirtAddr, size: PageSize) -> PagingResult<&mut PTE> {
        let _ = self.ensure_can_modify()?;

        if VirtPage::new_4k(vaddr).is_none() {
            return Err(PagingError::NotAligned);
        }

        let vaddr = *vaddr;

        let pt_l3 = if Arch::LEVELS == 3 {
            self.raw_table_of(self.root())?
        } else if Arch::LEVELS == 4 {
            let pt_l4 = self.raw_table_of(self.root())?;
            let pt_l4e = &mut pt_l4[Self::p4_index(vaddr)];
            self.get_create_next_level(pt_l4e)?
        } else {
            panic!("Unsupported page table");
        };

        let pt_l3e = &mut pt_l3[Self::p3_index(vaddr)];

        if size == PageSize::_1G {
            return Ok(pt_l3e);
        }

        let pt_l2 = self.get_create_next_level(pt_l3e)?;
        let pt_l2e = &mut pt_l2[Self::p2_index(vaddr)];
        if size == PageSize::_2M {
            return Ok(pt_l2e);
        }

        let p1 = self.get_create_next_level(pt_l2e)?;
        let p1e = &mut p1[Self::p1_index(vaddr)];
        Ok(p1e)
    }

    fn get_create_next_level<'a>(&mut self, entry: &mut PTE) -> PagingResult<&'a mut [PTE]> {
        let alloc = self.ensure_can_modify_mut()?;

        if entry.is_empty() {
            let frame = alloc
                .allocator
                .lock()
                .alloc_frame()
                .ok_or(PagingError::OutOfMemory)?;

            let paddr = frame.0;

            alloc.frames.push(frame);
            *entry = PTE::new_table(paddr);

            self.raw_table_of(paddr)
        } else {
            self.get_next_level(entry)
        }
    }
}

impl<Arch: IPageTableArchAttribute, PTE: IArchPageTableEntry> PageTableNative<Arch, PTE> {
    const NUM_ENTRIES: usize = 512;

    #[allow(unused)]
    #[inline(always)]
    const fn p4_index(vaddr: usize) -> usize {
        (vaddr >> (12 + 27)) & (Self::NUM_ENTRIES - 1)
    }

    #[inline(always)]
    const fn p3_index(vaddr: usize) -> usize {
        (vaddr >> (12 + 18)) & (Self::NUM_ENTRIES - 1)
    }

    #[inline(always)]
    const fn p2_index(vaddr: usize) -> usize {
        (vaddr >> (12 + 9)) & (Self::NUM_ENTRIES - 1)
    }

    #[inline(always)]
    const fn p1_index(vaddr: usize) -> usize {
        (vaddr >> 12) & (Self::NUM_ENTRIES - 1)
    }
}
