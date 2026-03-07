/*!
    Internal module defining a region-aware bump allocator

    The sole purpose of this module is to facilicate boot-time
    use of dynamically-allocated structures like [`Vec<T>`]
    and [`Box<T>`].

    [`Vec<T>`]: alloc::vec::Vec
    [`Box<T>`]: alloc::boxed::Box
*/

// Standard definitions
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

// Internal definitions
use common::shared::GenericError;
use common::shared::mm::{MemoryRegion, MemoryRegionKind, PhysMemRegion};
use common::shared::structs::spin_lock::Mutex;

// Helper routine: round up given address to the nearest aligned address
#[inline(always)]
#[doc(hidden)]
fn round_addr(base: usize, align: usize) -> usize {
    if base % align == 0 {
        base
    } else {
        base + align - (base % align)
    }
}

/**
    A "memory region kind" descriptor specifically for boot image regions
*/
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct BootImage {
    protected: usize,
    usable: usize,
}

impl BootImage {
    pub const fn new(protected: bool, usable: bool) -> Self {
        // - ooo... spooky ABI boundaries...
        BootImage {
            protected: if protected { 1 } else { 0 },
            usable: if usable { 1 } else { 0 },
        }
    }

    pub fn try_reclaim(&mut self) -> Result<bool, ()> {
        if self.protected == 1 {
            Err(())
        } else {
            let old = self.usable;
            self.usable = 1;
            Ok(old == 1)
        }
    }

    pub unsafe fn unset_protected(&mut self) {
        self.protected = 0;
    }
}

impl MemoryRegionKind for BootImage {
    fn is_usable(&self) -> bool {
        self.protected == 0 && self.usable == 1
    }

    fn is_reclaimable(&self) -> bool {
        self.protected == 0
    }
}

/// Type alias for a region within the boot image
pub type BootImageRegion = MemoryRegion<BootImage>;

/// Bump allocator state
pub(crate) struct BumpAllocatorState<T: Into<PhysMemRegion> + Copy + 'static> {
    pub phys_mem_layout: Option<&'static [T]>,
    pub logical_mem_layout: Option<&'static [BootImageRegion]>,
    pub base: usize,
    pub head: usize,
    pub remaining: usize,
}

impl<T: Into<PhysMemRegion> + Copy + 'static> BumpAllocatorState<T> {
    pub const fn new() -> Self {
        BumpAllocatorState {
            phys_mem_layout: None,
            logical_mem_layout: None,
            base: 0,
            head: 0,
            remaining: 0,
        }
    }
}

/**
    Region-aware bump allocator

    # Safety
    This allocator assumes direct access to physical memory, meaning that
    paging must either be disabled, or configured for identity-mapping.
*/
// - decide whether map entries should be copied or borrowed
// - decide whether we need concurrency guarantees (we probably don't)
// - decide whether we should account for paging (we probably don't,
//   as memory is identity-mapped elsewhere in the pipeline)
// TODO: finish prototype
pub struct BumpAllocator<T: Into<PhysMemRegion> + Copy + 'static> {
    state: Mutex<BumpAllocatorState<T>>,
}

impl<T: Into<PhysMemRegion> + Copy + 'static> BumpAllocator<T> {
    /**
        Create new instance of `BumpAllocator`

        May only be invoked once under `#[global_allocator]`
    */

    // - logical memory layout to be used in later revisions
    pub const fn new() -> Self {
        BumpAllocator {
            state: Mutex::new(BumpAllocatorState::new()),
        }
    }

    /**
        Return base address of current memory region
    */
    pub fn base(&self) -> usize {
        self.state.lock().base
    }

    /**
        Return allocator head address
    */
    pub fn head(&self) -> usize {
        self.state.lock().head
    }

    /**
        Return remaining region size
    */
    pub fn remaining(&self) -> usize {
        self.state.lock().remaining
    }

    /**
        Initialize allocator instance

        # Usage
        One can set `min_capacity = 0` to select whatever available
        region that is first encountered in the memory layout.

        # Safety
        Although this function is safe to call, it is the caller's
        responsibility to make sure that `phys_mem_layout` points to a
        valid memory layout, and that the entries genuinely reflect
        the system's current memory state (memory map, paging, etc.)
    */
    pub fn init(
        &self,
        phys_mem_layout: &'static [T],
        min_capacity: usize,
        logical_mem_layout: &'static [BootImageRegion],
    ) -> Result<(), GenericError> {
        // 0. Obtain handle to inner state
        let mut state = self.state.lock();
        state.logical_mem_layout = Some(logical_mem_layout);

        // 1. Use the provided logical memory layout
        // to locate a suitable physical memory region
        let mut candidate_region: Option<PhysMemRegion> = None;

        // - perform linear search, skipping the LMA
        for &e in phys_mem_layout {
            let entry: PhysMemRegion = e.into();

            // - skip areas that are either unusable, or those
            //   that overlap with the boot image
            if !entry.kind().is_usable() || logical_mem_layout.iter().any(|r| entry.overlaps(r)) {
                continue;
            }

            // - store the first candidate region
            // larger than or equal to `min_capacity`
            if entry.size() as usize >= min_capacity {
                candidate_region = Some(entry);
                break;
            }
        }

        // 3. Set allocator base
        // - if no suitable region was found, panic (why though?)
        if let Some(r) = candidate_region {
            let region: PhysMemRegion = r.into();
            let region_base = region.base() as usize;
            let region_size = region.size() as usize;

            state.base = region_base;

            // For sanity's sake, align base to 16 bytes
            let padding = round_addr(region_base, 16) - region_base;
            let new_head = (region_base + padding) as usize;

            // - do not use obviously small regions
            if padding > region_size {
                return Err(GenericError::ErrorMessage(
                    "candidate region too small to obtain an aligned allocator base",
                ));
            }

            // Calculate new region size
            let new_size = (region_size - padding) as usize;

            // Store aligned head and adjusted region size
            state.head = new_head;
            state.remaining = new_size;
            state.phys_mem_layout = Some(phys_mem_layout);
        } else {
            return Err(GenericError::ErrorMessage(
                "no suitable region for allocator base was found",
            ));
        }

        // 4. Perform sanity check
        // - the base should NEVER be equal to zero, as we have
        //   been searching look
        if state.base == 0 || state.head == 0 || state.remaining == 0 {
            return Err(GenericError::ErrorMessage(
                "no changes were made to the allocator state",
            ));
        }

        Ok(())
    }
}

// TODO REVIEW + FIXME: WTF is this madness!?
// TODO: use our novel "memory region descriptors", so that we
//       don't need to know what "low memory area" is
unsafe impl<T: Into<PhysMemRegion> + Copy + 'static> GlobalAlloc for BumpAllocator<T> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // Obtain handle to inner state
        let mut state = self.state.lock();

        // Do not proceed if the allocator has not been initialized
        if state.phys_mem_layout.is_none() || state.logical_mem_layout.is_none() {
            return null_mut();
        }

        // Unwrap the maybe-references
        let phys_mem_layout = state.phys_mem_layout.unwrap();
        let logical_mem_layout = state.logical_mem_layout.unwrap();

        // Obtain requested region size and alignment
        let req_size = layout.size();
        let req_align = layout.align();

        // Calculate aligned head
        let mut req_head = round_addr(state.head, req_align);

        // - since zero-size allocations are allowed, handle it
        if req_size == 0 {
            return req_head as *mut u8;
        }

        // Keep track of the old head
        // - `self.head` should be greater
        //    than the old head on success
        let old_head = state.head;

        // First-pass heuristic:
        // Can we still allocate within the current region,
        // subject to alignment requirements?
        let padding = req_head - old_head;
        if req_size + padding < state.remaining {
            // If so, EASY! perform a trivial "bump"
            state.remaining -= req_size + padding;
            state.head = req_head + req_size;
            return req_head as *mut u8;
        }

        // If not through each entry, ruling out regions
        // that
        // - are clearly unusable
        // - are clearly below the current base
        // - are split between "usable" and "non-usable"
        for &e in phys_mem_layout {
            let entry: PhysMemRegion = e.into();

            // Memory region properties
            let mem_base = entry.base() as usize;
            let mem_size = entry.size() as usize;
            let mem_end = mem_base + mem_size;
            let mem_pad = round_addr(mem_base, req_align) - mem_base;

            // - rule out those that overlap with the boot image
            if logical_mem_layout.iter().any(|r| entry.overlaps(r)) {
                continue;
            }

            // - rule out "past" regions
            if mem_base < state.base {
                continue;
            }

            // - rule out regions that are clearly unusable
            if mem_size <= req_size + mem_pad || !entry.kind().is_usable() {
                continue;
            }

            // - if the requested region is clearly below
            //   the current memory region, then use the
            //   memory region's base as the candidate base,
            //   subject to alignment requirements
            if req_head + req_size < mem_base {
                // - store region base
                state.base = mem_base;

                // - calculate new candidate base
                req_head = mem_base + mem_pad;

                // - calculate new capacity
                state.remaining = mem_size - mem_pad;
            }

            // - if the requested region is clearly contained
            //   within the current memory region, then
            //   we're done here; we can simply "bump" the
            //   base by the requested size + padding
            if req_head >= mem_base && mem_end >= req_head + req_size {
                // Bump base and capacity
                // FIXME: potential correctness issue here
                state.head = req_head + req_size;
                state.remaining -= req_size + mem_pad;
                break;
            }
        }

        // Return candidate base only if `self.base`
        // has changed values. Otherwise, return
        // a null pointer.
        if state.head > old_head {
            req_head as *mut u8
        } else {
            null_mut()
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        /* no-op */
    }
}

unsafe impl<T: Into<PhysMemRegion> + Copy + 'static> Sync for BumpAllocator<T> {}
unsafe impl<T: Into<PhysMemRegion> + Copy + 'static> Send for BumpAllocator<T> {}
