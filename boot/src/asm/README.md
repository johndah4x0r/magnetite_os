# magnetite_os/boot - x86 assembly sub-division
This document should serve as a breakdown over the semantics
of the assembly part of the **magnetite_os** bootloader.

**Note** that the semantics documented here, if any, are
provisional, as side-effects are being discovered and
expansions are being sketched out.

## Lessons learnt

### Mismatch between file size and assumed memory layout leading to failed reads
As the stage-2 loader binary `boot1.bin` grew in size, the VBR ran into issues related to
segmentation and low-level arithmetic. For instance, the routine responsible for loading
`boot1.bin` into memory initially assumed a flat-memory layout - even though *real mode*
uses segmentation, and cannot handle offsets greater than 65535. Additionally, a bug was
left undiscovered in the routine responsible for reading from the boot device until
`boot1.bin` grew so large that it spanned more than two clusters, thus leading to
seemingly-mysterious read failures.

_A sizeable amount of effort was dedicated to revising the VBR, with particular focus on
rigorous validation of arithmetic sub-routines, as well as a localized segmentation-first
model for key memory operations such as file loading. These changes, alogn with the
rationale that lead to them, should be formally documented, so as to prevent similar
outcomes._

### Overlap between second-stage loader and page tables leading to high-level panics and processor faults
As the Rust portion of `magnetite_os/boot` grew larger and more sophisticated, it ran into serious issues
regarding memory layout. For instance, adjusting the dimensions and in-struct location of the shadow
buffer used inside a VGA text mode helper struct ended up producing panics and triple-faults that were
hard to reason with, at least until *file size* and *memory location* were considered. 

_Significant effort was dedicated to revising the VBR and the stage-2 stubs, with particular
focus on changing the memory layout from an **address-based static layout** to an
**append-only dynamically-assessed** layout. These changes, along with the rationale that led
to them, should be formally documented, so as to prevent similar outcomes._

## Memory layout
The exact memory layout is laid out *ad hoc* in `defs.asm`.

### Firmware-constrained region
The bootloader should *at the minimum* guarantee the following
memory layout in order for further assumptions to be made:

| Region start | Region end   | Description                                                       | Size (IEC B) |
|:------------:|:------------:|:-----------------------------------------------------------------:|:------------:|
|`0x00000`     |`0x00FFF`     | IVT and BDA, along with padding to nearest small-page boundary    | 4096         |
|`0x01000`     |`0x07AFF`     | Bootstrap stack at `INIT_STACK := 0x07B00`                        | 27 392       |
|`0x07B00`     |`0x07BFF`     | Stack underflow limit; utilized by stack frame at `INIT_FRAME`    | 256          |
|`0x07C00`     |`0x07DFF`     | Stage-1 bootloader code; should not be mutated unless necessary   | 512          |

Note that the stage-2 loader expects the bootstrap stack to be usable.

### Transitional region
| Region start | Region end   | Description                                                       | Size (IEC B) |
|:------------:|:------------:|:-----------------------------------------------------------------:|:------------:|
|`0x07E00`     |`0x08BFF`     | Read buffer used by `vbr.bin`                                     | 4096         |
|`0x08E00`     |`0x08FFF`     | Buffer overflow limit / padding to nearest small-page boundary    | 512          |
|`0x09000`     |`0x3FFFF` / ? | *Loader-derived region*                                           | 220k / ?     |

### Loader-derived region (dynamic)
The memory space for the stage-2 loader should be configured as follows:

| Region symbol                                            | Description                              | Size                                            |
|:--------------------------------------------------------:|:----------------------------------------:|:-----------------------------------------------:|
|`ADDR_S2_LDR := 0x9000`                                   | Stage-2 bootloader base                  |`_sizeof_s2_ldr` (provided by linker)            |
|`[e820_map] := align(ADDR_S2_LDR + _sizeof_s2_ldr, 16)`   | E820 memory map descriptor + entries     | min. descriptor (16 B), max. descriptor + `E820_ENTRIES` map entries|
|`[page_structs] := align([e820_map] + 16 + SIZEOF_E820_ENTRY * [[e820_map] + 8], 4096)` | Bootstrap paging structures | min. 16 kiB (PML4, PDPT, PDT, PT) |

In simpler terms, the ordering should be

- the stage-2 loader binary, followed by
- the E820 map descriptor, along with E820 map entries, followed by
- bootstrap paging structures, which may expand as necessary

subject to the following address alignment requirements:

- structures **should** generally be aligned to 8-byte or 16-byte boundaries
- paging structures **must** be aligned to 4096-byte boundaries

## `defs.asm` - constant definitions
*The exact definitions, as well as their descriptions, can be found in* `defs.asm`.

### VBR constants
| Constant label              | Expression / value            | Description                                                             |
|:---------------------------:|:-----------------------------:|:-----------------------------------------------------------------------:|
| `START_VECTOR`              | `0x7C00`                      | **VBR start vector**                                                    |
| `SIZEOF_MAGIC`              | `3`                           | Size of the magic sequence `EB 3C 90`                                   |
| `OEM_LABEL`                 | `START_VECTOR + SIZEOF_MAGIC` | Location of the DOS 4.0 EBPB in memory, starting with the OEM label.    |
| `INIT_STACK`                | `0x7B00`                      | Initial value for the stack pointer `SP`                                |
| `ADDR_S2_LDR`               | `0x9000`                      | **Location of the stage-2 loader in memory**                            |
| `E820_ENTRIES`              | `1024`                        | Maximum number of E820 map entries to be queried                        |
| `SIZEOF_E820_ENTRY`         | `24`                          | Size of a single E820 entry                                             |
| `E820_DESC_ADDR`            | `0`                           | Offset to the address field in the E820 map descriptor                  |
| `E820_DESC_SIZE`            | `8`                           | Offset to the size field in the E820 map descriptor                     |
| `E820_DESC_END`             | `16`                          | Offset to the first E820 map entry (right after the descriptor)         |
| `SIZEOF_RDENTRY`            | `32`                          | Size of a FAT16 root directory entry                                    |
| `SIZEOF_83NAME`             | `11`                          | Size of a 8.3 filename                                                  |
| `FRAME`                     | `-20`                         | Start of the global variables frame (relative to `INIT_FRAME`)          |
| `DAP_FRAME`                 | `FRAME - 16`                  | Start of the DAP / boot drive reader frame                              |
| `LOWEST_FRAME`              | `DAP_FRAME`                   | Offset for the "lowest" frame                                           |
| `GUARD_SIZE`                | `2`                           | Number of bytes to reserve past the "lowest" frame                      |
| `ALLOC_SIZE`                | `-LOWEST_FRAME + GUARD_SIZE`  | Stack frame size                                                        |
| `INIT_FRAME`                | `INIT_STACK + ALLOC_SIZE`     | Initial value for the frame pointer `BP` (should be below stack bottom) |
| `RDE_FIRST_CLUSTER`         | `26`                          | Offset for cluster ID 0 in a root directory entry                       |

### Global variables frame
| Variable name               | Field location                | Description                                                          |
|:---------------------------:|:-----------------------------:|:--------------------------------------------------------------------:|
| `ROOT_DIR_SECTORS`          | `FRAME + 0`                   | root directory sectors count                                         |
| `FIRST_FAT_SECTORS_LOW`     | `FRAME + 2`                   | low word of the first FAT sector LBA                                 |
| `FIRST_FAT_SECTORS_HIGH`    | `FRAME + 4`                   | high word of the first FAT sector LBA                                |
| `FIRST_RD_SECTOR_LOW`       | `FRAME + 6`                   | low word of the first root directory sector LBA                      |
| `FIRST_RD_SECTOR_HIGH`      | `FRAME + 8`                   | high word of the first root directory sector LBA                     |
| `FIRST_DATA_SECTOR_LOW`     | `FRAME + 10`                  | low word of the first data sector LBA                                |
| `FIRST_DATA_SECTOR_HIGH`    | `FRAME + 12`                  | high word of the first data sector LBA                               |
| `LAST_ACCESSED_LOW`         | `FRAME + 14`                  | low word of the last-accessed sector LBA                             |
| `LAST_ACCESSED_HIGH`        | `FRAME + 16`                  | high word of the last-accessed sector LBA                            |
| `BOOT_DEV`                  | `FRAME + 18`                  | boot device number                                                   |

### DAP / boot drive reader frame
The BIOS expects the following fields to be placed at their respective offsets:

| Variable name               | Field location                | Description                                                          |
|:---------------------------:|:-----------------------------:|:--------------------------------------------------------------------:|
| `DAP_SIZE`                  | `DAP_FRAME + 0`               | Size of the Disk Address Packet. Must be equal to `0x10`.            |
| `DAP_NUM_SECTORS`           | `DAP_FRAME + 2`               | Number of sectors to request from the BIOS                           |
| `DAP_BUF_OFFSET`            | `DAP_FRAME + 4`               | Offset part of the pointer to target buffer                          |
| `DAP_BUF_SEGMENT`           | `DAP_FRAME + 6`               | Segment part of the pointer to target buffer                         |
| `DAP_LBA_LOW`               | `DAP_FRAME + 8`               | Lowest word of the target sector LBA                                 |
| `DAP_LBA_MID1`              | `DAP_FRAME + 10`              | Low-middle word of the target sector LBA                             |
| `DAP_LBA_MID2`              | `DAP_FRAME + 12`              | High-middle word of the target sector LBA (unused; kept at `0x0000`) |
| `DAP_LBA_HIGH`              | `DAP_FRAME + 14`              | Highest word of the target sector LBA (unused; kept at `0x0000`)     |

### CPU stage change constants
| Constant label              | Expression / value            | Description                                                                         |
|:---------------------------:|:-----------------------------:|:-----------------------------------------------------------------------------------:|
| `ID_EFLAGS`                 | `1 << 21`                     | `ID` bit ìn `EFLAGS` (unused; `CPUID` asserted by brute force)                      |
| `EXT_CPUID`                 | `1 << 31`                     | Value for `EAX` to check which `CPUID` extensions are present                       |
| `FEAT_CPUID`                | `1 << 31 \| 1`                | Value for `EAX` to query which processor extensions are present                     |
| `LM_EDX_CPUID`              | `1 << 29`                     | Value of the `LM` bit in `EDX` after invoking `CPUID` with `EAX = FEAT_CPUID`       |
| `NO_PAGING`                 | `(1 << 31) - 1`               | Bit mask to unset `CR0.PG` and disable paging                                       |
| `PAE_ENABLE`                | `1 << 5`                      | Value which sets `CR4.PAE` and enables Physical Address Expansion                   |
| `PG_ENABLE`                 | `1 << 31`                     | Value which sets `CR0.PG` and enables paging                                        |
| `EFER_MSR`                  | `0xC0000080`                  | Address for the `IA32_EFER` model-specific-register (MSR)                           |
| `EFER_LME`                  | `1 << 8`                      | Value which sets `IA32_EFER.LME` and enables IA32e Compatibility Mode / "long mode" |

### Paging structures
The stage-2 loader and the CPU expect the paging structures to be laid out as follows:

| Offset label                | Expression / value            | Description                                                                         |
|:---------------------------:|:-----------------------------:|:-----------------------------------------------------------------------------------:|
| `OFFSET_PTS`, `OFFSET_PML4` | `0`                           | Offset to paging structures (starting with the PML4)                                |
| `OFFSET_PDPT`               | `OFFSET_PML4 + SIZEOF_PT`     | Offset to a single PDPT relative to a calculated base                               |
| `OFFSET_PDT`                | `OFFSET_PDPT + SIZEOF_PT`     | Offset to a single PDT relative to a calculted base                                 |
| `OFFSET_PT`                 | `OFFSET_PDT + SIZEOF_PT`      | Offset to a single PT relative to a calculated base                                 |

The base is small page-aligned, and is determined by the location and size of the E820 map.

### Page table entry values and constants
| Constant label              | Expression / value            | Description                                                                         |
|:---------------------------:|:-----------------------------:|:-----------------------------------------------------------------------------------:|
| `SIZEOF_PT`, `SIZEOF_PAGE`  | `1 << 12`                     | Size of a small page and a page table (4096 B)                                      |
| `PT_ADDR_MASK`              | `(1 << 64) - (1 << 12)`       | Mask to align addresses to 4096 B                                                   |
| `PT_PRESENT`                | `1 << 0`                      | Value to mark a page table entry as *present*                                       |
| `PT_READWRITE`              | `1 << 1`                      | Value to mark a page table entry as *read/write*                                    |
| `PT_PAGESIZE`               | `1 << 7`                      | Value to mark a page table entry as *large/huge*                                    |
| `ENTRIES_PER_PT`            | `512`                         | Number of entries per page table                                                    |
| `SIZEOF_PT_ENTRY`           | `8`                           | Size of a page table entry                                                          |

### GDT selector access bits
| Constant label              | Expression / value            | Description                                                                         |
|:---------------------------:|:-----------------------------:|:-----------------------------------------------------------------------------------:|
| `SEG_PRESENT`               | `1 << 7`                      | Value to mark a segment as *present*                                                |
| `SEG_NOT_SYS`               | `1 << 4`                      | Value to mark a segment as a *non-system segment*                                   |
| `SEG_EXEC`                  | `1 << 3`                      | Value to mark a segment as *executable*                                             |
| `SEG_RW`                    | `1 << 1`                      | Value to mark a segment as *read/write*                                             |

### GDT selector flags
| Constant label              | Expression / value            | Description                                                                         |
|:---------------------------:|:-----------------------------:|:-----------------------------------------------------------------------------------:|
| `SEG_GRAN_4K`               | `1 << 7`                      | Value to set a segment's granularity to 4096 B                                      |
| `SEG_SZ_32`                 | `1 << 6`                      | Value to set a segment's default parameter width to 32-bit                          |
| `SEG_LONG_MODE`             | `1 << 5`                      | Value to mark a segment as a *long mode segment*                                    |

### Uncategorized constants
| Constant label              | Expression / value            | Description                                                                         |
|:---------------------------:|:-----------------------------:|:-----------------------------------------------------------------------------------:|
| `NULL`                      | `0`                           | Null pointer                                                                        |

## `vbr.asm` / `vbr.bin` - custom FAT16-aware 8086-safe volume boot record
_The VBR is, in its current state, sufficiently stable, and shall therefore be "frozen" in behavor; only patches concerning correctness fixes will be considered for future application._

### Surface-level information
The responsibilities of the VBR are as follows:

1. Stabilize CPU state
    - zero out segment registers to enforce a flat-memory model
    - initialize stack at `INIT_STACK`
    - initialize stack frame at `INIT_FRAME`
2. Stabilize boot drive state
    - store boot drive number in the variable `BOOT_DEV`
    - check whether the BIOS exposes the **Enhanced Disk Device** interface
3. Compute file system geometry
    - locate the first FAT sector,
    - the first root directory sector, and
    - the first data sector
4. Locate stage-2 loader executable (`boot1.bin`)
    - obtain a sample of the root directory, then
    - check whether an entry in the sample contains the 8.3 filename `BOOT1   BIN`, then
    - obtain more samples until the file is found, or until the root directory has been exhausted
5. Load stage-2 loader executable into memory
    - obtain the ID for the first cluster in the chain, then
    - calculate the location of the corresponding data sector, then
    - read the data sector to `ADDR_S2_LDR` using a sliding write head, then
    - calculate where the next cluster ID is located in the FAT, then
    - obtain a sample from FAT if necessary, then
    - follow the cluster chain, repeating the preceiding steps until end-of-chain (cluster ID g.t. `0xFFEF`) is encountered
6. Transfer control to stage-2 loader
    - copy boot device number from the variable `BOOT_DEV` into a known register, then
    - perform an intra-segment jump to `ADDR_S2_LDR`

The exact sequence of events can be found in `vbr.asm`.

### Breakdown of subroutines
TODO

(freehand: no sub-routines other than `read_bootdev` should be allowed to calculate buffer cursor position,
not least because registers are capped to `0xFFFF`, but also because segmentation is a PITA: `addr = (segment << 4) + offset`,
and making *all* routines calculate their own segments and offsets would kill all hopes of cramming FAT16-aware logic within
440-something bytes)

(freehand: since `compute_sectors`, `walk_rootdir`, `parse_entry` and `read_bootdev` all rely on FLAGS awareness
and mnemonic discipline, manual validation is effectively mandatory; one misplaced `xor` can make "smart" logic
look "dumb")

## `boot1.bin` - custom stage-2 bootloader
TODO

### `stub16.asm` - 16-bit stub for stage-2 bootloader
TODO

### `stub32.asm` - 32-bit stub for stage-2 bootloader
TODO

### `stub64.asm` - 64-bit stub for stage-2 bootloader
TODO

