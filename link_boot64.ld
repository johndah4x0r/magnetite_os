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

    . = ALIGN(16);

    /* Data-like */
    .boot64.data : SUBALIGN(1) {
        . = ALIGN(16);
        *(.rodata .rodata.*)

        . = ALIGN(16);
        *(.data .data.*)
    }

    . = ALIGN(16);
    .boot64.bss : SUBALIGN(1) {
        *(.bss .bss.*)
    }

    . = ALIGN(16);
}