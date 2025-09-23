use core::cell::OnceCell;

use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};

use crate::{AreaType, MapType, MappingArea, MappingAreaAllocation};
use address::{PhysAddr, VirtAddr, VirtAddrRange, VirtPage, VirtPageRange};
use allocation_abstractions::IFrameAllocator;
use hermit_sync::SpinMutex;
use mmu_abstractions::{GenericMappingFlags, PageSize, IMMU};

pub struct MemorySpace {
    mmu: Arc<SpinMutex<dyn IMMU>>,
    mapping_areas: Vec<MappingArea>,
    attr: OnceCell<MemorySpaceAttribute>,
    allocator: Arc<SpinMutex<dyn IFrameAllocator>>,
}

#[derive(Debug, Clone, Copy)]
pub struct MemorySpaceAttribute {
    pub brk_area_idx: usize,
    pub brk_start: VirtAddr,
    pub stack_guard_base: VirtAddrRange,
    pub stack_range: VirtAddrRange,
    pub stack_guard_top: VirtAddrRange,
    pub elf_area: VirtAddrRange,
    pub signal_trampoline: VirtPage,
}

impl Default for MemorySpaceAttribute {
    /// Creates a default MemorySpaceAttribute with all address ranges set to null and numeric fields set to sentinel values.
    ///
    /// The returned value is suitable as an uninitialized placeholder:
    /// - `brk_area_idx` is `usize::MAX` (indicating no brk area assigned),
    /// - `brk_start`, `stack_guard_base`, `stack_range`, `stack_guard_top`, and `elf_area` are all empty/null ranges,
    /// - `signal_trampoline` is `0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use memory_space::MemorySpaceAttribute;
    ///
    /// let attr = MemorySpaceAttribute::default();
    /// assert_eq!(attr.brk_area_idx, usize::MAX);
    /// assert!(attr.brk_start.is_null());
    /// assert_eq!(*attr.signal_trampoline.addr(), 0);
    /// ```
    fn default() -> Self {
        Self {
            brk_area_idx: usize::MAX,
            brk_start: VirtAddr::null,
            stack_guard_base: VirtAddrRange::new(VirtAddr::null, VirtAddr::null),
            stack_range: VirtAddrRange::new(VirtAddr::null, VirtAddr::null),
            stack_guard_top: VirtAddrRange::new(VirtAddr::null, VirtAddr::null),
            elf_area: VirtAddrRange::new(VirtAddr::null, VirtAddr::null),
            signal_trampoline: VirtPage::new_4k(VirtAddr::null).unwrap(),
        }
    }
}

impl MemorySpace {
    pub fn mappings(&self) -> &[MappingArea] {
        &self.mapping_areas
    }

    pub fn alloc_and_map_area(&mut self, mut area: MappingArea) {
        debug_assert!(area.allocation.is_none());

        let mut alloc = self.create_empty_area_allocation();

        {
            for vpn in area.range().iter() {
                let frame = alloc.allocator.lock().alloc_frame().unwrap();
                let paddr = frame.0;

                alloc.frames.insert(vpn, frame);

                self.mmu
                    .lock()
                    .map_single(vpn.addr(), paddr, PageSize::_4K, area.permissions())
                    .unwrap();
            }
        }

        area.allocation = Some(alloc);
        self.mapping_areas.push(area);
    }

    pub fn map_area(&mut self, area: MappingArea) {
        debug_assert!(area.allocation.is_some());
        debug_assert!(Arc::ptr_eq(
            &area.allocation.as_ref().unwrap().allocator,
            &self.allocator
        ));

        self.mapping_areas.push(area);
    }

    pub fn unmap_first_area_that(&mut self, predicate: &impl Fn(&MappingArea) -> bool) -> bool {
        match self.mapping_areas.iter().position(predicate) {
            Some(index) => {
                let area = self.mapping_areas.remove(index);
                for vpn in area.range.iter() {
                    self.mmu.lock().unmap_single(vpn.addr()).unwrap();
                }
                // Drop area to release allocated frames
                true
            }
            None => false,
        }
    }

    pub fn unmap_all_areas_that(&mut self, predicate: impl Fn(&MappingArea) -> bool) {
        while self.unmap_first_area_that(&predicate) {
            // do nothing
        }
    }

    pub fn unmap_area_starts_with(&mut self, vpn: VirtPage) -> bool {
        self.unmap_first_area_that(&|area| area.range.start() == vpn)
    }
}

impl MemorySpace {
    pub fn attr(&self) -> &MemorySpaceAttribute {
        self.attr.get().unwrap()
    }

    pub fn brk_start(&self) -> VirtAddr {
        self.attr().brk_start
    }

    pub fn brk_page_range(&self) -> VirtPageRange {
        self.mapping_areas[self.brk_area_idx()].range()
    }

    pub fn brk_area_idx(&self) -> usize {
        self.attr().brk_area_idx
    }

    pub fn increase_brk(&mut self, new_end_vpn: VirtPage) -> Result<(), &str> {
        let brk_idx = self.brk_area_idx();

        let old_end_vpn;

        {
            let brk_area = &mut self.mapping_areas[brk_idx];

            if new_end_vpn < brk_area.range.start() {
                return Err("New end is less than the current start");
            }

            old_end_vpn = brk_area.range.end();
        }

        let page_count = new_end_vpn.diff_page_count(old_end_vpn);

        if page_count == 0 {
            return Ok(());
        }

        let increased_range = VirtPageRange::new(old_end_vpn, page_count as usize);

        for vpn in increased_range.iter() {
            let frame = self.allocator.lock().alloc_frame().unwrap();
            let paddr = frame.0;

            let area = &mut self.mapping_areas[brk_idx];

            area.allocation.as_mut().unwrap().frames.insert(vpn, frame);

            self.mmu
                .lock()
                .map_single(vpn.addr(), paddr, PageSize::_4K, area.permissions())
                .unwrap();
        }

        let brk_area = &mut self.mapping_areas[brk_idx];

        brk_area.range =
            VirtPageRange::from_start_end(brk_area.range.start(), new_end_vpn).unwrap();

        Ok(())
    }
}

impl MemorySpace {
    pub fn new(
        mmu: Arc<SpinMutex<dyn IMMU>>,
        allocator: Arc<SpinMutex<dyn IFrameAllocator>>,
    ) -> Self {
        Self {
            mmu,
            mapping_areas: Vec::new(),
            attr: OnceCell::new(),
            allocator,
        }
    }

    pub fn mmu(&self) -> &Arc<SpinMutex<dyn IMMU>> {
        &self.mmu
    }

    pub fn allocator(&self) -> &Arc<SpinMutex<dyn IFrameAllocator>> {
        &self.allocator
    }

    pub(crate) fn create_empty_area_allocation(&self) -> MappingAreaAllocation {
        MappingAreaAllocation {
            allocator: self.allocator.clone(),
            frames: BTreeMap::new(),
        }
    }

    /// Initialize the memory space's attribute value
    ///
    /// # Safety
    ///
    /// The function is NOT thread safe.
    pub unsafe fn init(&mut self, attr: MemorySpaceAttribute) {
        self.attr.set(attr).unwrap();
    }
}

impl MemorySpace {
    // Clone the existing memory space
    pub fn clone_existing(
        them: &MemorySpace,
        mmu: Arc<SpinMutex<dyn IMMU>>,
        allocator: Option<Arc<SpinMutex<dyn IFrameAllocator>>>,
    ) -> Self {
        let mut this = Self::new(mmu, allocator.unwrap_or(them.allocator().clone()));

        let mut buffer: [u8; constants::PAGE_SIZE] = [0; constants::PAGE_SIZE];

        for area in them.mapping_areas.iter() {
            let my_area = MappingArea::clone_from(area);
            this.alloc_and_map_area(my_area);

            // Copy datas through high half address
            for src_page in area.range.iter() {
                let their_pt = them.mmu().lock();

                their_pt.read_bytes(src_page.addr(), &mut buffer).unwrap();

                this.mmu()
                    .lock()
                    .write_bytes(src_page.addr(), &buffer)
                    .unwrap();
            }
        }

        *this.attr.get_mut().unwrap() = *them.attr();

        this
    }

    pub fn signal_trampoline(&self) -> VirtPage {
        self.attr().signal_trampoline
    }

    pub fn register_signal_trampoline(&mut self, sigreturn: PhysAddr) {
        const PERMISSIONS: GenericMappingFlags = GenericMappingFlags::Kernel
            .union(GenericMappingFlags::User)
            .union(GenericMappingFlags::Readable)
            .union(GenericMappingFlags::Executable);

        log::info!("Registering signal trampoline at {:?}", sigreturn);

        assert!(!self.signal_trampoline().addr().is_null());

        let trampoline_page = self.signal_trampoline();

        self.mmu
            .lock()
            .map_single(
                trampoline_page.addr(),
                sigreturn,
                PageSize::_4K,
                PERMISSIONS,
            )
            .unwrap();

        self.mapping_areas.push(MappingArea::new(
            VirtPageRange::new(trampoline_page, 1),
            AreaType::SignalTrampoline,
            MapType::Framed,
            PERMISSIONS,
            None,
        ));
    }
}
