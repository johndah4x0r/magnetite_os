/*!
    Structures specific to the x86 PC/BIOS platform
*/

use core::ptr;

use crate::shared::mm::{PhysMemKind, PhysMemRegion};

/// Short (20 B) E820 entry
// - expect little-endian encoding (x86-exclusive)
// - this can be represented normally, as we
//   handle integer reconstruction ourselves
#[derive(Debug, Copy, Clone)]
#[repr(C, align(4))]
pub struct ShortE820 {
    _base_low: u32,
    _base_high: u32,
    _size_low: u32,
    _size_high: u32,
    _area_type: u32,
}

impl ShortE820 {
    /// Return region base
    pub const fn base(&self) -> u64 {
        ((self._base_high as u64) << 32) | (self._base_low as u64)
    }

    /// Return region size
    pub const fn size(&self) -> u64 {
        ((self._size_high as u64) << 32) | (self._size_low as u64)
    }

    /// Return area type
    pub const fn area_type(&self) -> u32 {
        self._area_type
    }
}

impl From<LongE820> for ShortE820 {
    fn from(value: LongE820) -> ShortE820 {
        let base: u64 = value.base();
        let size: u64 = value.size();
        let area_type: u32 = value.area_type();

        ShortE820 {
            _base_low: base as u32,
            _base_high: (base >> 32) as u32,
            _size_low: size as u32,
            _size_high: (size >> 32) as u32,
            _area_type: area_type,
        }
    }
}

impl From<ShortE820> for PhysMemRegion {
    fn from(value: ShortE820) -> PhysMemRegion {
        let base = value.base() as usize;
        let size = value.size() as usize;
        let kind = match value.area_type() {
            1 => PhysMemKind::Regular,
            2 => PhysMemKind::Reserved(None),
            3 => PhysMemKind::Reclaimable(None),
            4 => PhysMemKind::NonVolatile(None),
            _ => PhysMemKind::Other(None),
        };

        PhysMemRegion::new(base, size, kind)
    }
}

/// Long (24 B) E820 entry
// - expect little-endian encoding (x86-exclusive)
// - this can be represented normally
//   (as in, without language-level packing)
#[derive(Debug, Copy, Clone)]
#[repr(C, align(8))]
pub struct LongE820 {
    _base: u64,
    _size: u64,
    _area_type_attr: u64,
}

impl LongE820 {
    /// Return region base
    pub const fn base(&self) -> u64 {
        self._base
    }

    /// Return region size
    pub const fn size(&self) -> u64 {
        self._size
    }

    /// Return area type
    pub const fn area_type(&self) -> u32 {
        self._area_type_attr as u32
    }

    /// Return ACPI attributes
    pub const fn acpi_attr(&self) -> u32 {
        (self._area_type_attr >> 32) as u32
    }
}

impl From<LongE820> for PhysMemRegion {
    fn from(value: LongE820) -> PhysMemRegion {
        let base = value.base() as usize;
        let size = value.size() as usize;
        let attr = value.acpi_attr() as usize;
        let kind = match value.area_type() {
            1 => PhysMemKind::Regular,
            2 => PhysMemKind::Reserved(Some(attr)),
            3 => PhysMemKind::Reclaimable(Some(attr)),
            4 => PhysMemKind::NonVolatile(Some(attr)),
            _ => PhysMemKind::Other(Some(attr)),
        };

        PhysMemRegion::new(base, size, kind)
    }
}

/// Structure representing a DOS 4.0 BIOS parameter block (EBPB)
// - if it should ever become necessary
#[repr(C, packed)]
pub struct BiosPB {
    _oem_label_raw: [u8; 8],
    _bytes_per_sector: u16,
    _sectors_per_cluster: u8,
    _reserved_sectors: u16,
    _fat_count: u8,
    _root_dir_entries: u16,
    _sectors: u16,
    _medium_type: u8,
    _sectors_per_fat: u16,
    _heads: u8,
    _hidden_sectors: u32,
    _large_sectors: u32,
    _drive_number: u16,
    _signature: u8,
    _volume_id: u32,
    _volume_label: [u8; 11],
    _filesystem: [u8; 8],
}

// - should we "normalize" all numerical quantities to `usize`?
impl BiosPB {
    /// Return number of bytes per sector
    pub fn bytes_per_sector(&self) -> usize {
        unsafe { ptr::read_unaligned(&raw const self._bytes_per_sector) as usize }
    }

    /// Return number of sectors per cluster
    pub fn sectors_per_cluster(&self) -> usize {
        self._sectors_per_cluster as usize
    }

    /// Return number of reserved sectors
    pub fn reserved_sectors(&self) -> usize {
        unsafe { ptr::read_unaligned(&raw const self._reserved_sectors) as usize }
    }

    /// Return number of FATs
    pub fn fat_count(&self) -> usize {
        self._fat_count as usize
    }

    /// Return number of root directory entries
    pub fn root_dir_entries(&self) -> usize {
        unsafe { ptr::read_unaligned(&raw const self._root_dir_entries) as usize }
    }

    /// Return number of sectors
    ///
    /// If the return value is equal to zero, use [`large_sectors()`] instead
    ///
    /// [`large_sectors()`]: Self::large_sectors
    pub fn sectors(&self) -> usize {
        unsafe { ptr::read_unaligned(&raw const self._sectors) as usize }
    }

    /// Return medium type
    pub fn medium_type(&self) -> usize {
        self._medium_type as usize
    }

    /// Return number of sectors per FAT
    pub fn sectors_per_fat(&self) -> usize {
        unsafe { ptr::read_unaligned(&raw const self._sectors_per_fat) as usize }
    }

    /// Return number of drive heads
    pub fn heads(&self) -> usize {
        self._heads as usize
    }

    /// Return number of hidden sectors
    pub fn hidden_sectors(&self) -> usize {
        unsafe { ptr::read_unaligned(&raw const self._hidden_sectors) as usize }
    }

    /// Return number of sectors
    ///
    /// If the return value is equal to zero, use [`sectors()`] instead
    ///
    /// [`sectors()`]: Self::sectors
    pub fn large_sectors(&self) -> usize {
        unsafe { ptr::read_unaligned(&raw const self._large_sectors) as usize }
    }

    /// Return volume signature
    pub fn signature(&self) -> usize {
        self._signature as usize
    }

    /// Return volume ID
    pub fn volume_id(&self) -> usize {
        unsafe { ptr::read_unaligned(&raw const self._volume_id) as usize }
    }
}
