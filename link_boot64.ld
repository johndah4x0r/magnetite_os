/*
    Linker script for stitching together
    'stub64.o' and 'boot_rs.o' into
    'boot64.bin'

    We'll strive for position independence.
*/

ENTRY(_start)            /* make sure _start exists */

SECTIONS {
    /* Text + data (flat binary) */
    .text : SUBALIGN(1) {
        KEEP(*(.stub64))

        . = ALIGN(16); 
        *(.text .text.*)
        *(.rodata .rodata.*)
        *(.data .data.*)
    }
}