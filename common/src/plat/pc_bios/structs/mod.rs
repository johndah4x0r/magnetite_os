/*!
    Structures specific to the x86 PC/BIOS platform
*/

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
    pub fn base(&self) -> u64 {
        ((self._base_high as u64) << 32) | (self._base_low as u64)
    }

    /// Return region size
    pub fn size(&self) -> u64 {
        ((self._size_high as u64) << 32) | (self._size_low as u64)
    }

    /// Return area type
    pub fn area_type(&self) -> u32 {
        self._area_type
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
    pub fn base(&self) -> u64 {
        self._base
    }

    /// Return region size
    pub fn size(&self) -> u64 {
        self._size
    }

    /// Return area type
    pub fn area_type(&self) -> u32 {
        self._area_type_attr as u32
    }

    /// Return ACPI attributes
    pub fn acpi_attr(&self) -> u32 {
        (self._area_type_attr >> 32) as u32
    }
}

/// BIOS parameter block structure
// - if it should ever become necessary
#[repr(C, packed)]
pub struct BiosPB {
    oem_label_raw: [u8; 8],
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    fat_count: u8,
    root_dir_entries: u16,
    sectors: u16,
    medium_type: u8,
    sectors_per_fat: u16,
    heads: u8,
    hidden_sectors: u32,
    large_sectors: u32,
    drive_number: u16,
    signature: u8,
    volume_id: u32,
    volume_label: [u8; 11],
    filesystem: [u8; 8],
}
