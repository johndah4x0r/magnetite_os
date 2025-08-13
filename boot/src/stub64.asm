[bits 64]

; Do not use absolute positioning, as the
; binaries are linked at a later stage
global _start           ; Global export of _start
extern main             ; Import of Rust 'main' routine

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
    ; - assuming ECX is preserved mid-jump,
    ;   we should be able to see the data
    ;   segment number in ECX
    mov ds, cx
    mov es, cx
    mov fs, cx
    mov gs, cx
    mov ss, cx

    ; Reset stack
    mov rsp, 0x7b00
    mov rbp, rsp

    ; Dereference RDI and RDX to unwrap
    ; the "contained" values
    ;
    ; The contract is as follows:
    ; - RDI: pointer to OEM label pointer,
    ;        into OEM label pointer
    ; - RSI: pointer to boot drive number,
    ;        into boot drive number
    ; - RDX: pointer to E820 map pointer,
    ;        into E820 map pointer
    ; - (RCX: zero-extended data segment number)
    mov rdi, [rdi]
    mov rsi, [rsi]
    mov rdx, [rdx]

    ; TODO

.spin:
    ; Freeze without eternally halting
    ; (to avoid killing debuggers...)
    pause               ; Signal spin loop
    jmp .spin           ; Repeat