[bits 64]

; Do not use absolute positioning, as the
; binaries are linked at a later stage
global _start

section .boot64

; Use '_start' to appease linker, and to
; assert the importance of this stub for
; 64-bit operation of the bootloader
_start:
    ; TODO
    cli
    xchg rbx, rbx
    hlt