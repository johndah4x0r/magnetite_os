/*
    Linker script for stitching together
    'boot64_wrap.o' and 'stub32.o' into
    'boot1.bin'
*/

ENTRY(_stub32)                      /* make sure _stub32 exists */

SECTIONS {
    . = 0x8000;                     /* hard-coded entry point in MBR */

    _start_offset = _start - .;     /* 32-bit offset to 64-bit entry */

    /* Text + data (flat binary) */
    .text : SUBALIGN(1) {
        /* 
            Alias start of 'boot64.bin' 
            Highly sensitive to source path
        */

        *(.text .text.*)
        *(.data .data.*)

        /*
            Embedded 64-bit loader is a
            second-class citizen
        */
        . = ALIGN(16);
        *(.w_text)
    }
}