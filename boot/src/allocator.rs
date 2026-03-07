// Standard definitions
use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::null_mut;

// Internal definitions
use common::shared::GenericError;
use common::shared::mm::{LogicalMemRegion, PhysMemRegion};

// Helper routine: round up given address to the nearest aligned address
#[inline(always)]
fn round_addr(base: usize, align: usize) -> usize {
    if base % align == 0 {
        base
    } else {
        base + align - (base % align)
    }
}

/// Bump allocator that uses a memory layout provided by E820
// - decide whether map entries should be copied or borrowed
// - decide whether we need to know ACPI attributes (necessitating
//   the use of `LongE820`)
// - decide whether we need concurrency guarantees (we probably don't)
// - decide whether we should account for paging (we probably don't,
//   as memory is identity-mapped elsewhere in the pipeline)
// TODO: finish prototype
// FIXME: generalize, so that we aren't using x86-isms
// TODO: use our novel "memory region descriptors", so that we
//       don't need to know what "low memory area" is
#[repr(C)]
pub struct BumpAllocator<T: Into<PhysMemRegion> + Copy + 'static> {
    _phys_mem_layout: UnsafeCell<Option<&'static [T]>>,
    _logical_mem_layout: UnsafeCell<Option<&'static [LogicalMemRegion]>>,
    _base: UnsafeCell<usize>,
    _head: UnsafeCell<usize>,
    _remaining: UnsafeCell<usize>,
}

impl<T: Into<PhysMemRegion> + Copy + 'static> BumpAllocator<T> {
    /**
        Create new instance of `BumpAllocator`

        May only be invoked once under `#[global_allocator]`
    */

    // - logical memory layout to be used in later revisions
    pub const fn new() -> Self {
        BumpAllocator {
            _phys_mem_layout: UnsafeCell::new(None),
            _logical_mem_layout: UnsafeCell::new(None),
            _base: UnsafeCell::new(0),
            _head: UnsafeCell::new(0),
            _remaining: UnsafeCell::new(0),
        }
    }

    // Internal: obtain optional mutable reference to physical memory layout
    pub fn phys_mem_layout(&self) -> &mut Option<&'static [T]> {
        unsafe { &mut *self._phys_mem_layout.get() }
    }

    // Internal: obtain mutable reference to region base
    pub fn base(&self) -> &mut usize {
        unsafe { &mut *self._base.get() }
    }

    // Internal: obtain mutable reference to allocator head
    pub fn head(&self) -> &mut usize {
        unsafe { &mut *self._head.get() }
    }

    // Internal: obtain mutable reference to remaining region space
    pub fn remaining(&self) -> &mut usize {
        unsafe { &mut *self._remaining.get() }
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
    ) -> Result<(), GenericError> {
        // 1. Use the provided logical memory layout
        // to locate a suitable physical memory region
        let mut candidate_region: Option<PhysMemRegion> = None;

        // - perform linear search, skipping the LMA
        for &e in phys_mem_layout {
            let entry: PhysMemRegion = e.into();

            // - skip LMA (region start below 1 MiB) and unavailable areas
            if (entry.base() < (1 << 20)) || !entry.is_usable() {
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

            *self.base() = region_base;

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
            *self.head() = new_head;
            *self.remaining() = new_size;
            *self.phys_mem_layout() = Some(phys_mem_layout);
        } else {
            return Err(GenericError::ErrorMessage(
                "no suitable region for allocator base was found",
            ));
        }

        // 4. Perform sanity check
        // - the base should NEVER be equal to zero, as we have
        //   been searching look
        if *self.base() == 0 || *self.head() == 0 || *self.remaining() == 0 {
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
        // Do not proceed if the allocator has not been initialized
        if self.phys_mem_layout().is_none() {
            return null_mut();
        }

        // Obtain requested region size and alignment
        let req_size = layout.size();
        let req_align = layout.align();

        // Calculate aligned head
        let mut req_head = round_addr(*self.head(), req_align);

        // - since zero-size allocations are allowed, handle it
        if req_size == 0 {
            return req_head as *mut u8;
        }

        // Keep track of the old head
        // - `self.head` should be greater
        //    than the old head on success
        let old_head = *self.head();

        // First-pass heuristic:
        // Can we still allocate within the current region,
        // subject to alignment requirements?
        let padding = req_head - old_head;
        if req_size + padding < *self.remaining() {
            // If so, EASY! perform a trivial "bump"
            *self.remaining() -= req_size + padding;
            *self.head() = req_head + req_size;
            return req_head as *mut u8;
        }

        // If not through each entry, ruling out regions
        // that
        // - are clearly unusable
        // - are clearly below the current base
        // - are split between "usable" and "non-usable"
        for &e in self.phys_mem_layout().unwrap() {
            let entry: PhysMemRegion = e.into();

            // Memory region properties
            let mem_base = entry.base() as usize;
            let mem_size = entry.size() as usize;
            let mem_end = mem_base + mem_size;
            let mem_pad = round_addr(mem_base, req_align) - mem_base;

            // - rule out "past" regions
            if mem_base < *self.base() {
                continue;
            }

            // - rule out regions that are clearly unusable
            if mem_size < req_size + mem_pad || !entry.is_usable() {
                continue;
            }

            // - if the requested region is clearly below
            //   the current memory region, then use the
            //   memory region's base as the candidate base,
            //   subject to alignment requirements
            if req_head + req_size < mem_base {
                // - store region base
                *self.base() = mem_base;

                // - calculate new candidate base
                req_head = mem_base + mem_pad;

                // - calculate new capacity
                *self.remaining() = mem_size - mem_pad;
            }

            // - if the requested region is clearly contained
            //   within the current memory region, then
            //   we're done here; we can simply "bump" the
            //   base by the requested size + padding
            if req_head >= mem_base && mem_end >= req_head + req_size {
                // Bump base and capacity
                *self.head() = req_head + req_size;
                *self.remaining() -= req_size;
                break;
            }
        }

        // Return candidate base only if `self.base`
        // has changed values. Otherwise, return
        // a null pointer.
        if *self.head() > old_head {
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
