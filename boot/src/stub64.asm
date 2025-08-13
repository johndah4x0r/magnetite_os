[bits 64]

; Do not use absolute positioning, as the
; binaries are linked at a later stage
global _start           ; Global export of _start
extern main             ; Import of Rust 'main' routine

;extern _hal_offset      ; offset to HAL (defined by linker)

NULL    equ 0           ; Null pointer

section .stub64
; Use '_start' to appease linker, and to
; assert the importance of this stub for
; 64-bit operation of the bootloader
_start:
    ; Header (like, c'mon?)
    ; 0a. 32-bit near jump (e9 RR RR RR RR)
    ; 0b. NOP padding (90 90 90)
    ; 1.  64-bit relative vector to Rust 'main' routine
    ;     (VV VV VV VV VV VV VV VV)
    ; 2.  maybe-pointer to HAL vector table (NULL for now)
    ; 3.  NOP padding (90 90 90 90 90 90 90 90)
    jmp qword .start
    align 8, nop
.handover_offset:
    dq NULL
.vt_offset:
    dq NULL
.pad:
    align 16, nop
.start:
    cli                 ; Kill interrupts

    ; Initialize segments
    ; - assuming ECX is preserved mid-jump,
    ;   we should be able to see the data
    ;   segment number in ECX
    mov ds, cx
    mov es, cx
    mov fs, cx
    mov gs, cx
    mov ss, cx

    ; Dereference RDI and RDX to unwrap
    ; the "contained" values

.spin:
    ; Freeze without eternally halting
    ; (to avoid killing debuggers...)
    pause               ; Signal spin loop
    jmp .spin           ; Repeat