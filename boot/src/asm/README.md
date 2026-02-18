# magnetite_os/boot - x86 assembly sub-division
This document should serve as a breakdown over the semantics
of the assembly part of the **magnetite_os** bootloader.

**Note** that the semantics documented here, if any, are
provisional, as side-effects are being discovered and
expansions are being sketched out.

## Lessons learnt

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

### Holistic layout
The bootloader should *at the minimum* guarantee the following
memory layout in order for further assumptions to be made:

| Region start | Region end   | Description                                                       | Size (IEC B) |
|:------------:|:------------:|:-----------------------------------------------------------------:|:------------:|
|`0x00000`     |`0x00FFF`     | IVT and BDA, along with padding to nearest small-page boundary    | 4096         |
|`0x01000`     |`0x07AFF`     | Bootstrap stack at `INIT_STACK := 0x07B00`                        | 27 392       |
|`0x07B00`     |`0x07BFF`     | Stack underflow limit; utilized by stack frame at `INIT_FRAME`    | 256          |
|`0x07C00`     |`0x07DFF`     | Stage-1 bootloader code; should not be mutated unless necessary   | 512          |
|`0x07E00`     |`0x08BFF`     | Read buffer used by `vbr.bin`                                     | 4096         |
|`0x08E00`     |`0x08FFF`     | Buffer overflow limit / padding to nearest small-page boundary    | 512          |
|`0x09000`     |`0x3FFFF` / ? | Stage-2 bootloader code + E820 map + bootstrap page tables        | 220k / ?     |

The exact memory layout is laid out *ad hoc* in `defs.asm`.

### Stage-2 loader space

| Region symbol                                        | Description                              | Size                                            |
|:----------------------------------------------------:|:-----------------------------------------|:-----------------------------------------------:|
|`ADDR_S2_LDR := 0x9000`                               | Stage-2 bootloader base                  |`_sizeof_s2_ldr` (provided by linker)            |
|`[e820_map] := align(ADDR_S2_LDR + _sizeof_s2_ldr)`   | E820 memory map descriptor + entries     | min. descriptor (8 B), max. descriptor + `E820_ENTRIES` map entries|
|`[page_structs] := align([e820_map] + 8 + SIZEOF_E820_ENTRY * [[e820_map] + 8])` | Bootstrap paging structures | min. 4096 B (PML4, PDPT, PDT, PT) |

## `defs.asm` - constant definitions
TODO

## `vbr.asm` / `vbr.bin` - custom FAT16-aware 8086-safe volume boot record
TODO

## `boot1.bin` - custom stage-2 bootloader
TODO

### `stub16.asm` - 16-bit stub for stage-2 bootloader
TODO

### `stub32.asm` - 32-bit stub for stage-2 bootloader
TODO

### `stub64.asm` - 64-bit stub for stage-2 bootloader
TODO

