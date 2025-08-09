/*
    Linker script for stitching together
    'boot64.o' and 'boot_rs.o' into
    'boot64.bin'
*/

ENTRY(_start)            /* make sure _start exists */

SECTIONS {
    /* Text + data (flat binary) */
    .text : SUBALIGN(1) {
        *(.boot64)

        . = ALIGN(16); 
        *(.text .text.*)
        *(.rodata .rodata.*)
        *(.data .data.*)
    }
}