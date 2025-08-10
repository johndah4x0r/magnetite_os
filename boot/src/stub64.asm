[bits 64]

; Do not use absolute positioning, as the
; binaries are linked at a later stage
global _start

NULL    equ 0           ; Null pointer

section .stub64
; Use '_start' to appease linker, and to
; assert the importance of this stub for
; 64-bit operation of the bootloader
_start:
    ; Header (like, c'mon?)
    ; 0.  (9) 64-bit near jump (e9 RR RR RR RR RR? RR? RR? RR?)
    ; 1.  (3) zero padding (00 00 00)
    ; 2.  (8) 64-bit offset to 'main' (here: NULL)
    jmp qword .start
.pad:
    times 12-($-$$) db 0
.handover_offset:
    dq NULL
.start:
    cli                 ; Kill interrupts

    ; Initialize segments
    ; - assuming EAX is preserved mid-jump,
    ;   we should be able to see the data
    ;   segment number in EAX
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

.spin:
    ; Freeze without eternally halting
    ; (to avoid killing debuggers...)
    pause               ; Signal spin loop
    jmp .spin           ; Repeat