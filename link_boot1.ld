/*
    Linker script for stitching together
    'boot64.o' and 'stub32.o' into
    'boot1.bin'
*/

ENTRY(_stub16)                          /* make sure _stub16 exists */

SECTIONS {
    _addr_s2_ldr = 0x00009000;              /* hard-coded entry point in definitions file */
    . = _addr_s2_ldr;

    /* Text + data (flat binary) */
    .text : SUBALIGN(16) {
        *(.stub16)
        *(.text .text.*)
        *(.data .data.*)

        /*
            Embedded 64-bit loader is a
            second-class citizen
        */
        *(.boot64.text)
        *(.boot64.data)
    }

    /* End of the stage-2 loader space (including .bss)*/
    _base_s2_end = _bss_end;

    /* Size of stage-2 loader binary + padding for BSS (in bytes) */
    _sizeof_s2_ldr = _bss_start - _addr_s2_ldr;
    _sizeof_bss = _bss_end - _bss_start;
}
