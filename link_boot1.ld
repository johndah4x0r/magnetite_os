/*
    Linker script for stitching together
    'boot64_wrap.o' and 'stub32.o' into
    'boot1.bin'
*/

ENTRY(_stub16)                          /* make sure _stub32 exists */

SECTIONS {
    . = 0xf000;                         /* hard-coded entry point in definitions file */

    _stub64_offset = _stub64 - .;       /* offset to 64-bit entry */

    /* Text + data (flat binary) */
    .text : SUBALIGN(1) {
        *(.stub16)

        . = ALIGN(16);
        *(.text .text.*)

        . = ALIGN(16);
        *(.data .data.*)

        /*
            Embedded 64-bit loader is a
            second-class citizen
        */
        . = ALIGN(16);
        *(.boot64.text .boot64.data)
    }
}