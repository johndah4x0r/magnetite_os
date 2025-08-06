[bits 32]

; Do not use absolute positioning, as the
; binaries are linked at a later stage

; Preamble section
section .text.preamble
_preamble:
    ; Reuse legacy magic sequence
    jmp dword _boot32
    nop

    ; Align preamble to 16 B
    times 16-($-$$) nop

; ---- context ---- ;
; - we intend to place .text.hal
;   in between .text.preamble and
;   .text.boot
; - the vector list can be defined
;   in .data, so long as the
;   kernel can locate it later

section .text.boot32
_boot32:


; end of 32-bit code - start of 64-bit code
[bits 64]
section .text.boot64
_boot64:


; separating text from data isn't that
; crucial in binary executables, but
; it is best practice (we'll merge
; them anyways during linking)
section .data.preboot

; Pointer to BPB (32-bit pointer extended to 64-bit)
oem_label:
    .low    dd 0
    .high   dd 0

; Active text mode (16-bit extended to 64-bit)
textmode:
    .low    dd 0
    .high   dd 0

; Boot device (8-bit extended to 64-bit)
bootdev:
    .low    dd 0
    .high   dd 0