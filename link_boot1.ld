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
        *(.boot64.text)

        . = ALIGN(16);
        *(.boot64.data)

        . = ALIGN(4096);
        *(.boot64.bss)
    }

    /* End of the stage-2 loader */
    _base_s2_end = .;

    /* Size of stage-2 loader (in bytes) */
    _sizeof_s2_ldr = _base_s2_end - _addr_s2_ldr;
}
