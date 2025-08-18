[bits 64]

; Do not use absolute positioning, as the
; binaries are linked at a later stage
;
; This is especially important, as we want
; the secondary (tertiary?) stage to be
; independent of initial position.

global _start           ; Global export of _start

extern main             ; Import of Rust 'main' routine
extern __hal_offset     ; offset to HAL (defined by linker)

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
    dq main
.vt_offset:
    dq __hal_offset
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

    ; Reset stack
    ; - this is the only occasion where
    ; we'd be using static references
    mov rsp, 0x7b00
    mov rbp, rsp

    ; Dereference RDI and RSI to unwrap the
    ; "contained" values
    ;
    ; The contract is as follows:
    ; - RDI: absolute pointer to OEM label pointer,
    ;        into OEM label pointer
    ; - RSI: absolute pointer to boot drive number,
    ;        into boot drive number
    ; - RDX: absolute pointer to E820 map pointer,
    ;        into E820 map pointer
    ; - (RCX: zero-extended data segment number)

    ; - limit access to 32-bit space
    and rdi, 0xFFFFFFFF
    and rsi, 0xFFFFFFFF
    and rdx, 0xFFFFFFFF

    mov rdi, [rdi]
    mov rsi, [rsi]
    mov rdx, [rdx]

    ; TODO
    ; Call Rust main routine
    ; - NEVER perform a far call, as there's
    ; no need to change segments, and we'd
    ; otherwise run the risk of feeding RIP 
    ; with blatantly ncorrect values
    call main
    ; --- fall-through (unlikely) --- ;

.spin:
    ; Freeze without eternally halting
    ; (to avoid killing debuggers...)
    pause               ; Signal spin loop
    jmp .spin           ; Repeat