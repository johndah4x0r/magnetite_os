[bits 64]

; Do not use absolute positioning, as the
; binaries are linked at a later stage

section .text.boot64
_boot64:
    ; TODO
    cli
    hlt