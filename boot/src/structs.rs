// BIOS parameter block structure
// - if it shoud ever become necessary
#[repr(C, packed)]
pub struct BiosPB {
    oem_label_raw:          [u8; 8],
    bytes_per_sector:       u16,
    sectors_per_cluster:    u8,
    reserved_sectors:       u16,
    fat_count:              u8,
    root_dir_entries:       u16,
    sectors:                u16,
    medium_type:            u8,
    sectors_per_fat:        u16,
    heads:                  u8,
    hidden_sectors:         u32,
    large_sectors:          u32,
    drive_number:           u16,
    signature:              u8,
    volume_id:              u32,
    volume_label:           [u8; 11],
    filesystem:             [u8; 8],
}