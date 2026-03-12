/*!
    Definitions relevant to memory mapping and management

    This module doesn't implement memory management *per se*, but rather
    expose definitions that assist in implementing it elsewhere, as it
    primarily defines platform-agnostic *advisories*.
*/

/*
    Not quite sure where to put these, so I'll put them here for now...

    Also, what in the Java is this abomination!?
*/

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(usize)]
pub enum PhysMemClass {
    Invalid = 0,
    Regular = 1,
    Reserved = 2,
    Reclaimable = 3,
    NonVolatile = 4,
    Hole = 5,
    Other = 6,
}

/**
    Platform-agnostic classification of a region in physical memory

    # Background
    This abstraction is primarily inspired by the region classes
    typically found in the PC/BIOS platform, though their presence
    or use isn't strictly limited to that platform.

    # Usage
    This type is immutable and state-poor by design, as it would
    be unsafe to directly manipulate memory map entries provided
    by the firmware.

    If one really wants to manipulate a particular entry, then
    one has to overwrite the old instance with a new instance.
*/

// - not sure whether `bool` is truly FFI-safe... might use
//   `usize` later...
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct PhysMemKind {
    _class: PhysMemClass,
    raw_attr: usize,
    has_attr: usize,
}

impl PhysMemKind {
    pub const fn new(class: PhysMemClass, attr: Option<usize>) -> Self {
        // - ooo... spooky ABI guarantees...
        let has_attr: usize = if attr.is_some() { 1 } else { 0 };
        let raw_attr: usize = match attr {
            Some(a) => a,
            None => 0,
        };

        PhysMemKind {
            _class: class,
            raw_attr,
            has_attr,
        }
    }

    pub const fn regular() -> Self {
        Self::new(PhysMemClass::Regular, None)
    }

    pub const fn reserved(attr: Option<usize>) -> Self {
        Self::new(PhysMemClass::Reserved, attr)
    }

    pub const fn reclaimable(attr: Option<usize>) -> Self {
        Self::new(PhysMemClass::Reclaimable, attr)
    }

    pub const fn non_volatile(attr: Option<usize>) -> Self {
        Self::new(PhysMemClass::NonVolatile, attr)
    }

    pub const fn hole() -> Self {
        Self::new(PhysMemClass::Hole, None)
    }

    pub const fn other(attr: Option<usize>) -> Self {
        Self::new(PhysMemClass::Other, attr)
    }

    pub fn class(&self) -> PhysMemClass {
        self._class
    }

    pub fn attr(&self) -> Option<usize> {
        if self.has_attr == 1 {
            Some(self.raw_attr)
        } else {
            None
        }
    }
}

/**
    Trait that marks a type as a "memory region kind" descriptor
*/
pub trait MemoryRegionKind {
    fn is_usable(&self) -> bool;
    fn is_reclaimable(&self) -> bool;
}

impl MemoryRegionKind for PhysMemKind {
    fn is_usable(&self) -> bool {
        self.class() == PhysMemClass::Regular
    }

    fn is_reclaimable(&self) -> bool {
        self.class() == PhysMemClass::Reclaimable
    }
}

/**
    Platform-agnostic descriptor of an interval in memory

    This structure does not care about the platform's policy,
    and is only concerned with the geometry of memory.
*/
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct RegionSpan {
    _base: usize,
    _size: usize,
}

impl RegionSpan {
    /**
        Creates new instance of `RegionSpan`
    */
    pub const fn new(base: usize, size: usize) -> Self {
        RegionSpan {
            _base: base,
            _size: size,
        }
    }

    /**
        Returns the base address of the region being described
    */
    pub fn base(&self) -> usize {
        self._base
    }

    /**
        Returns the size (in bytes) of the region being described
    */
    pub fn size(&self) -> usize {
        self._size
    }

    /**
        Returns the limit of the region being described

        The returned address is the address of the final
        byte in the region, plus one byte. In other words,
        the address is the theoretical base of a subsequent
        region (or memory hole).

        # Semantics
        If garbage values were provided to [`new()`], such that
        `base() + size()` exceeds [`usize::MAX`], then the
        returned limit will be capped to [`usize::MAX`].

        [`new()`]: Self::new
    */
    pub fn limit(&self) -> usize {
        self._base.saturating_add(self._size)
    }

    /**
        Checks whether the described region contains the
        provided address, and returns a truth value
    */
    pub fn contains_addr(&self, addr: usize) -> bool {
        self.base() <= addr && addr < self.limit()
    }

    /**
        Checks whether the described region overlaps with the
        provded region, and returns a truth value
    */
    pub fn overlaps(&self, other: &Self) -> bool {
        self.base() < other.limit() && other.base() < self.limit()
    }

    /**
        Checks whether the provided region is fully contained
        within the described region, and returns a truth value
    */
    pub fn contains(&self, other: &Self) -> bool {
        self.base() <= other.base() && other.limit() <= self.limit()
    }

    /**
        Checks whether the described region is fully above the
        provided region, and returns a truth value
    */
    pub fn is_above(&self, other: &Self) -> bool {
        other.limit() <= self.base()
    }

    /**
        Checks whether the described region is fully below the
        provided region, and returns a truth value
    */
    pub fn is_below(&self, other: &Self) -> bool {
        self.limit() <= other.base()
    }
}

/**
    Platform-agnostic descriptor of a "memory region"

    # Background
    This descriptor aspires to be platform-agnostic, though
    some platform-specific semantics may still be present,
    and should therefore be accounted for in critical
    applications.

    The internal structure, and the semantics, of `MemoryRegion` are
    reminiscent of the memory map layout entries generated by E820h
    in the PC/BIOS platform, though this type is not obligated to
    adhere to ACPI specifications.
*/
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct MemoryRegion<K: MemoryRegionKind> {
    _span: RegionSpan,
    _kind: K,
}

impl<K: MemoryRegionKind> MemoryRegion<K> {
    /**
        Creates new instance of `MemoryRegion`

        One should not have to run `new()` manually; it should instaed be
        invoked internally by implementors of [`into::<MemoryRegion<K>>()`].

        [`into::<MemoryRegion<K>>()`]: Into::into
    */
    pub const fn new(base: usize, size: usize, kind: K) -> Self {
        MemoryRegion {
            _span: RegionSpan::new(base, size),
            _kind: kind,
        }
    }

    /**
        Returns a reference to the span of the region being described
    */
    pub fn span(&self) -> &RegionSpan {
        &self._span
    }

    /**
        Returns a reference to the memory kind descriptor
    */
    pub fn kind(&self) -> &K {
        &self._kind
    }

    /**
        Returns a mutable reference to the memory kind descriptor

        # Safety
        Manipulating a memory kind descriptor may be unsafe, depending
        on the type of memory being described.

        For instance, directly lying about physical memory is almost
        certainly unsafe, which is why manipulating physical memory
        region descriptors is highly discouraged (see [`PhysMemKind`]).
    */
    pub fn kind_mut(&mut self) -> &mut K {
        &mut self._kind
    }
}

/// Type alias for a physical memory region
pub type PhysMemRegion = MemoryRegion<PhysMemKind>;
