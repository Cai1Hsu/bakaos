use address::{VirtAddr, VirtAddrRange, VirtPage, VirtPageRange};
use alloc::{string::String, sync::Arc, vec::Vec};
use hermit_sync::SpinMutex;
use log::trace;
use memory_space::{AreaType, MapType, MappingArea, MemorySpace, MemorySpaceAttribute};
use mmu_abstractions::{GenericMappingFlags, IMMU};
use utilities::InvokeOnDrop;
use xmas_elf::{program::ProgramHeader, ElfFile};

use crate::{auxv::AuxVecKey, IExecSource, LinuxLoader, LoadError, ProcessContext, RawMemorySpace};

impl<'a> LinuxLoader<'a> {
    /// Load an ELF executable into a newly created MemorySpace and return a configured LinuxLoader.
    ///
    /// This:
    /// - allocates contiguous physical frames, copies the ELF bytes into them, and parses the ELF;
    /// - maps PT_LOAD segments into the process address space (with permissions derived from segment flags),
    ///   tracking the loaded ELF area and PHDR location (or deriving it from the ELF header);
    /// - populates the process auxiliary vector (AT_PHDR, AT_PHENT, AT_PHNUM, AT_PAGESZ, AT_BASE, AT_FLAGS, AT_ENTRY);
    /// - reserves a signal trampoline page and sets up stack regions (guard base, user stack, guard top) and a brk area;
    /// - computes the program entry point (accounting for PIE offset when applicable) and initializes the MemorySpace with the collected attributes.
    ///
    /// Notes:
    /// - The function will consume and return the provided ProcessContext in the resulting LinuxLoader.
    /// - `mmu` and `alloc` are used to allocate and map memory; they are not documented here as generic services.
    /// - MemorySpace::init is called unsafely to finalize the layout.
    ///
    /// Errors:
    /// - Returns Err(LoadError::InsufficientMemory) if contiguous frames cannot be allocated for the ELF.
    /// - Returns Err(LoadError::UnableToReadExecutable) if reading the executable into memory fails.
    /// - Returns Err(LoadError::NotElf) if the ELF parser rejects the data.
    /// - Returns Err(LoadError::TooLarge) or Err(LoadError::IncompleteExecutable) for invalid segment sizes/offsets.
    /// - Returns Err(LoadError::FailedToLoad) if writing segment bytes into the MMU fails.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use std::sync::Arc;
    /// use linux_loader::{LinuxLoader, ProcessContext, IExecSource, IMMU, IFrameAllocator};
    /// # // The following is illustrative; real types and setup are required to run.
    /// let elf: &dyn IExecSource = /* ... */;
    /// let ctx = ProcessContext::default();
    /// let mmu: Arc<_> = /* MMU instance */;
    /// let alloc: Arc<_> = /* frame allocator */;
    /// let loader = LinuxLoader::from_elf(elf, "/bin/app", ctx, &mmu, &alloc).expect("failed to load ELF");
    /// ```
    pub fn from_elf(
        elf_data: &impl IExecSource,
        path: &str,
        mut ctx: ProcessContext<'a>,
        memory_space: &RawMemorySpace,
    ) -> Result<Self, LoadError> {
        let (mmu, alloc) = memory_space;
        let mut memory_space = MemorySpace::new(mmu.clone(), alloc.clone());

        let mut attr = MemorySpaceAttribute::default();

        // see https://github.com/caiyih/bakaos/issues/26
        let boxed_elf_holding;

        let boxed_elf;

        let elf_info = {
            let required_frames = elf_data.len().div_ceil(constants::PAGE_SIZE);

            let frames = alloc
                .lock()
                .alloc_contiguous(required_frames)
                .ok_or(LoadError::InsufficientMemory)?;

            boxed_elf_holding = InvokeOnDrop::transform(frames, |f| alloc.lock().dealloc_range(f));

            let pt = mmu.lock();

            let slice = pt
                .translate_phys(
                    boxed_elf_holding.start().addr(),
                    boxed_elf_holding.as_addr_range().len(),
                )
                .unwrap();

            let len = elf_data
                .read_at(0, slice)
                .map_err(|_| LoadError::UnableToReadExecutable)?;

            boxed_elf = &mut slice[..len];

            ElfFile::new(boxed_elf).map_err(|_| LoadError::NotElf)?
        };

        // No need to check the ELF magic number because it is already checked in `ElfFile::new`
        // let elf_magic = elf_header.pt1.magic;
        // '\x7fELF' in ASCII
        // const ELF_MAGIC: [u8; 4] = [0x7f, 0x45, 0x4c, 0x46];

        let mut min_start_vpn =
            VirtPage::new_custom_unchecked(VirtAddr::new(usize::MAX), constants::PAGE_SIZE);
        let mut max_end_vpn = VirtPage::new_custom_unchecked(VirtAddr::null, constants::PAGE_SIZE);

        let mut implied_ph = VirtAddr::null;
        let mut phdr = VirtAddr::null;

        let mut interpreters = Vec::new();

        let mut pie_offset = 0;

        for ph in elf_info.program_iter() {
            trace!("Found program header: {ph:?}");

            match ph.get_type() {
                Ok(xmas_elf::program::Type::Load) => trace!("Loading"),
                Ok(xmas_elf::program::Type::Interp) => {
                    interpreters.push(ph);
                    trace!("Handle later");
                    continue;
                }
                Ok(xmas_elf::program::Type::Phdr) => {
                    phdr = VirtAddr::new(ph.virtual_addr() as usize);
                    trace!("Handled");
                    continue;
                }
                _ => {
                    trace!("skipping");
                    continue;
                }
            }

            let mut start = VirtAddr::new(ph.virtual_addr() as usize);
            let mut end = start + ph.mem_size() as usize;

            let start_page = VirtPage::new_aligned_4k(start);
            let end_page = VirtPage::new_aligned_4k(end.align_up(constants::PAGE_SIZE)); // end is exclusive

            if start_page.page_num() == 0 {
                pie_offset = constants::PAGE_SIZE;
            }

            if pie_offset != 0 {
                start += pie_offset;
                end += pie_offset;
            }

            if implied_ph.is_null() {
                implied_ph = start;
            }

            min_start_vpn = min_start_vpn.min(start_page);
            max_end_vpn = max_end_vpn.max(end_page);

            let mut segment_permissions = GenericMappingFlags::User | GenericMappingFlags::Kernel;

            if ph.flags().is_read() {
                segment_permissions |= GenericMappingFlags::Readable;
            }

            if ph.flags().is_write() {
                segment_permissions |= GenericMappingFlags::Writable;
            }

            if ph.flags().is_execute() {
                segment_permissions |= GenericMappingFlags::Executable;
            }

            let page_range = VirtPageRange::from_start_end(
                start_page, end_page, // end is exclusive
            )
            .unwrap();

            memory_space.alloc_and_map_area(MappingArea::new(
                page_range,
                AreaType::UserElf,
                MapType::Framed,
                segment_permissions,
                None,
            ));

            fn copy_elf_segment(
                elf: &[u8],
                ph: &ProgramHeader,
                vaddr: VirtAddr,
                mmu: &Arc<SpinMutex<dyn IMMU>>,
            ) -> Result<(), LoadError> {
                let file_sz = ph.file_size() as usize;

                if file_sz > 0 {
                    let off = ph.offset() as usize;
                    let end = off.checked_add(file_sz).ok_or(LoadError::TooLarge)?;
                    if end > elf.len() {
                        return Err(LoadError::IncompleteExecutable);
                    }
                    let data = &elf[off..end];
                    mmu.lock()
                        .write_bytes(vaddr, data)
                        .map_err(|_| LoadError::FailedToLoad)?;
                }

                Ok(())
            }

            copy_elf_segment(boxed_elf, &ph, start, mmu)?;
        }

        for interp in interpreters {
            log::warn!("interpreter found: {interp:?}")
            // TODO
        }

        debug_assert!(min_start_vpn.page_num() > 0);

        attr.elf_area = VirtAddrRange::new(min_start_vpn.addr(), max_end_vpn.addr());

        log::debug!("Elf segments loaded, max_end_vpn: {max_end_vpn:?}");

        if phdr.is_null() {
            phdr = implied_ph + elf_info.header.pt2.ph_offset() as usize
        }

        ctx.auxv.insert(AuxVecKey::AT_PHDR, *phdr);
        ctx.auxv.insert(
            AuxVecKey::AT_PHENT,
            elf_info.header.pt2.ph_entry_size() as usize,
        );
        ctx.auxv
            .insert(AuxVecKey::AT_PHNUM, elf_info.header.pt2.ph_count() as usize);
        ctx.auxv.insert(AuxVecKey::AT_PAGESZ, constants::PAGE_SIZE);
        ctx.auxv.insert(AuxVecKey::AT_BASE, 0); // FIXME: correct value
        ctx.auxv.insert(AuxVecKey::AT_FLAGS, 0);
        ctx.auxv.insert(
            AuxVecKey::AT_ENTRY, // always the main program's entry point
            elf_info.header.pt2.entry_point() as usize,
        );

        // Reserved for signal trampoline
        max_end_vpn += 1;
        attr.signal_trampoline = max_end_vpn;

        max_end_vpn += 1;
        memory_space.alloc_and_map_area(MappingArea::new(
            VirtPageRange::new(max_end_vpn, 1),
            AreaType::UserStackGuardBase,
            MapType::Framed,
            GenericMappingFlags::empty(),
            None,
        ));
        attr.stack_guard_base = max_end_vpn.as_range();

        let stack_page_count = constants::USER_STACK_SIZE / constants::PAGE_SIZE;
        max_end_vpn += 1;
        memory_space.alloc_and_map_area(MappingArea::new(
            VirtPageRange::new(max_end_vpn, stack_page_count),
            AreaType::UserStack,
            MapType::Framed,
            GenericMappingFlags::User
                .union(GenericMappingFlags::Readable)
                .union(GenericMappingFlags::Writable),
            None,
        ));
        attr.stack_range = max_end_vpn.as_range();

        max_end_vpn += stack_page_count;
        let stack_top = max_end_vpn.addr();
        memory_space.alloc_and_map_area(MappingArea::new(
            VirtPageRange::new(max_end_vpn, 1),
            AreaType::UserStackGuardTop,
            MapType::Framed,
            GenericMappingFlags::empty(),
            None,
        ));
        attr.stack_guard_top = max_end_vpn.as_range();

        max_end_vpn += 1;
        memory_space.alloc_and_map_area(MappingArea::new(
            VirtPageRange::new(max_end_vpn, 0),
            AreaType::UserBrk,
            MapType::Framed,
            GenericMappingFlags::User
                .union(GenericMappingFlags::Readable)
                .union(GenericMappingFlags::Writable),
            None,
        ));
        attr.brk_area_idx = memory_space
            .mappings()
            .iter()
            .enumerate()
            .find(|(_, area)| area.area_type == AreaType::UserBrk)
            .expect("UserBrk area not found")
            .0;
        attr.brk_start = max_end_vpn.addr();

        // FIXME: handle cases where there is a interpreter
        let entry_pc = VirtAddr::new(elf_info.header.pt2.entry_point() as usize) + pie_offset;

        #[cfg(debug_assertions)]
        {
            for area in memory_space.mappings() {
                let area_type = area.area_type;
                let area_range = area.range;

                log::trace!("{area_type:?}: {area_range:?}");
            }

            let trampoline_page = attr.signal_trampoline;
            log::trace!("SignalTrampoline: {trampoline_page:?}");
        }

        unsafe {
            memory_space.init(attr);
        }

        Ok(LinuxLoader {
            memory_space,
            entry_pc,
            stack_top,
            argv_base: stack_top,
            envp_base: stack_top,
            ctx,
            executable: String::from(path),
        })
    }
}
