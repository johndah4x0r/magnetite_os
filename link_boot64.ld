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
    .boot64.text : SUBALIGN(1) {
        KEEP(*(.stub64))

        . = ALIGN(16);

        /* offset is guaranteed to be positive */
        __hal_offset = .;
        KEEP(*(.vt_hal))

        . = ALIGN(16);
        KEEP(*(.hal.text .hal.data))

        . = ALIGN(16);
        *(.text .text.*)
    }

    . = ALIGN(16);

    /* Data-like */
    .boot64.data : SUBALIGN(1) {
        *(.rodata .rodata.*)

        . = ALIGN(16);
        *(.data .data.*)
    }
}