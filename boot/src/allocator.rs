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
use common::shared::mm::{MemoryRegion, MemoryRegionKind, PhysMemRegion, RegionSpan};
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
    pub phys_mem_layout: &'static [T],
    pub logical_mem_layout: &'static [BootImageRegion],
    pub current_arena: RegionSpan,
}

impl<T: Into<PhysMemRegion> + Copy + 'static> BumpAllocatorState<T> {
    /**
        (internal) Locate new arena using the provided
        request parameters
    */
    pub(crate) fn locate_new_arena(&mut self, req_size: usize, req_align: usize) -> Result<(), ()> {
        // Obtain inner descriptors
        let phys_mem_layout = self.phys_mem_layout;
        let logical_mem_layout = self.logical_mem_layout;

        // - copy old arena descriptor
        let mut current_arena = self.current_arena;
        let mut new_arena: Option<RegionSpan> = None;

        // Locate new arena, ruling out physical regions that
        // - are clearly unusable
        // - are clearly below the current base
        // - are split between "usable" and "non-usable"
        for &e in phys_mem_layout {
            let entry: PhysMemRegion = e.into();
            let entry_span = entry.span();

            // Obtain region base and padding size
            let mem_base = entry_span.base();
            let mem_size = entry_span.size();
            let mem_pad = round_addr(mem_base, req_align) - mem_base;

            // - rule out those that overlap with the boot image
            if logical_mem_layout
                .iter()
                .any(|r| entry_span.overlaps(r.span()))
            {
                continue;
            }

            // - rule out "past" regions
            if entry_span.is_below(&current_arena) {
                continue;
            }

            // - rule out regions that are either too small
            //   or aren't explicitly declared as "usable"
            if entry_span.size() <= req_size + mem_pad || !entry.kind().is_usable() {
                continue;
            }

            // - if the requested region is clearly below
            //   the current memory region, then use the
            //   memory region's base as the candidate base,
            //   subject to alignment requirements
            if current_arena.is_below(entry.span()) {
                // - align entry base, then
                // break the loop
                new_arena = Some(RegionSpan::new(mem_base + mem_pad, mem_size - mem_pad));

                break;
            }
        }

        if let Some(a) = new_arena {
            // - store new arena
            self.current_arena = a;

            // - return `Ok(())`
            Ok(())
        } else {
            // - possible OOM; return `Err(())`
            Err(())
        }
    }
}

/**
    Region-aware bump allocator

    # Semantics
    The allocator state is private and stored behind a mutex lock.
    Allocations may therefore block. This is so that the allocator
    state is changed atomically.

    If the current arena satisfies the requested layout, allocation
    proceeds as usual: head pointer is "bumped", and capacity is
    decremented. If not, then the arena is considered to be *exhausted*,
    and the allocator performs a first-fit search over physical memory
    regions that
    - are "above" the current arena, and
    - are large enough to accomodate the requested layout, subject
      to alignment requirements

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
    state: Mutex<Option<BumpAllocatorState<T>>>,
}

impl<T: Into<PhysMemRegion> + Copy + 'static> BumpAllocator<T> {
    /**
        Create new instance of `BumpAllocator`

        May only be invoked once under `#[global_allocator]`
    */

    // - logical memory layout to be used in later revisions
    pub const fn new() -> Self {
        BumpAllocator {
            state: Mutex::new(None),
        }
    }

    /**
        Initialize allocator instance

        # Usage
        One can set `min_capacity = 0` to select whatever available
        region that is first encountered in the memory layout.

        # Semantics
        The allocator state is queried, and then initialized, atomically:
        no changes will be made to it until an iniitial state has been
        successfully calculated.

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
        let mut inner = self.state.lock();

        // - do not proceed if the allocator is already initialized
        if inner.is_some() {
            return Err(GenericError::ErrorMessage(
                "attempted to initialize allocator more than once",
            ));
        }

        // 1. Use the provided logical memory layout
        // to locate a suitable physical memory region
        let mut candidate_region: Option<PhysMemRegion> = None;

        // - perform linear search, skipping the LMA
        for &e in phys_mem_layout {
            let entry: PhysMemRegion = e.into();
            let entry_span = entry.span();

            // - skip areas that are either unusable, or those
            //   that overlap with the boot image
            if !entry.kind().is_usable()
                || logical_mem_layout
                    .iter()
                    .any(|r| entry_span.overlaps(r.span()))
            {
                continue;
            }

            // - store the first candidate region
            // larger than or equal to `min_capacity`
            if entry_span.size() >= min_capacity {
                candidate_region = Some(entry);
                break;
            }
        }

        // 3. Set allocator base
        // - if no suitable region was found, panic (why though?)
        if let Some(r) = candidate_region {
            let region: PhysMemRegion = r.into();
            let region_base = region.span().base();
            let region_size = region.span().size();

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
            *inner = Some(BumpAllocatorState {
                phys_mem_layout,
                logical_mem_layout,
                current_arena: RegionSpan::new(new_head, new_size),
            });
        } else {
            return Err(GenericError::ErrorMessage(
                "no suitable region for allocator base was found",
            ));
        }

        // 4. Perform sanity check
        // - the base should NEVER be equal to zero, as we have
        //   been searching look
        if inner.is_none() {
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
    /*
        Bump allocation with optional arena relocation
    */
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // 0. Obtain handle to inner state
        let mut inner = self.state.lock();

        // - do not proceed if the allocator has not been initialized
        if inner.is_none() {
            return null_mut();
        }

        // - unwrap inner state
        let state = inner.as_mut().unwrap();

        /* 1. Handle requested layout */

        // Keep track of the old head
        let mut old_head = state.current_arena.base();

        // Obtain requested region size and alignment
        let req_size = layout.size();
        let req_align = layout.align();

        // Calculate aligned head
        let mut req_head = round_addr(old_head, req_align);
        let mut padding = req_head - old_head;

        // - since zero-size allocations are allowed, handle it
        if req_size == 0 {
            return req_head as *mut u8;
        }

        /* 2. Perform optional arena relocation */

        // Can we still allocate within the current region,
        // subject to alignment requirements?
        // - if not, locate new region
        if state.current_arena.size() < req_size + padding {
            if state.locate_new_arena(req_size, req_align).is_err() {
                // - if a new arena couldn't be located, return NULL
                return null_mut();
            }
        }

        /* 3. Perform classic bump allocation */

        // Re-calculate request parameters, in case the arena has relocated
        old_head = state.current_arena.base();
        req_head = round_addr(old_head, req_align);
        padding = req_head - old_head;

        // Perform a trivial bump within the current arena
        let new_remaining = state.current_arena.size() - (req_size + padding);
        let new_head = req_head + req_size;

        // - instead of changing the fields in-place,
        //   store a new instance
        state.current_arena = RegionSpan::new(new_head, new_remaining);

        // - return pointer to requested head
        return req_head as *mut u8;
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        /* no-op */
    }
}

unsafe impl<T: Into<PhysMemRegion> + Copy + 'static> Sync for BumpAllocator<T> {}
unsafe impl<T: Into<PhysMemRegion> + Copy + 'static> Send for BumpAllocator<T> {}
