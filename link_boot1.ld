/*
    Linker script for stitching together
    'boot64_wrap.o' and 'boot32.o' into
    'boot1.bin'
*/

ENTRY(_boot32)            /* make sure _boot32 exists */

SECTIONS {
    . = 0x8000;             /* hard-coded entry point in MBR */

    /* Text + data (flat binary) */
    .text : SUBALIGN(1) {
        /* alias start of 'boot64.bin' */
        _start = _binary_boot64_bin_start;

        *(.text .text.*)
        *(.data .data.*)

        . = ALIGN(16);
        *(.w_text)
    }
}