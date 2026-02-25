/*
    Linker script for stitching together
    'stub64.o' and 'boot_rs.o' into
    'boot64.o'

    Do not peg '.' to a fixed address,
    as we'll be outputting yet another
    object file
*/

ENTRY(_stub64)            /* make sure _stub64 exists */

SECTIONS {
    /* Text-like */
    .boot64.text : SUBALIGN(16) {
        KEEP(*(.stub64))
        *(.text .text.*)
    }

    /* Data-like */
    .boot64.data : SUBALIGN(16) {
        *(.rodata .rodata.*)
        *(.data .data.*)
    }

    /* Zero-initialized regions */
    _bss_start = .;
    .boot64.bss : SUBALIGN(16) {
        *(.bss.allocator)
        *(.bss .bss.*)
    }

    . = ALIGN(16);
    _bss_end = .;
}