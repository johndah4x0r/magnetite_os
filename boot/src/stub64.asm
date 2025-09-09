[bits 64]

; Do not use absolute positioning, as the
; binaries are linked at a later stage
;
; This is especially important, as we want
; the secondary (tertiary?) stage to be
; independent of initial position.

global _stub64          ; Global export of _stub64
extern main             ; Import of Rust 'main' routine

NULL    equ 0           ; Null pointer

section .stub64
_stub64:
    ; Header (like, c'mon?)
    ; 0.  (9) 64-bit near jump (e9 RR RR RR RR RR? RR? RR? RR?)
    ; 1.  (3) zero padding (00 00 00)
    ; 2.  (8) 64-bit offset to 'main'
    jmp qword .start
.pad:
    times 12-($-$$) db 0
.handover_offset:
    dq main - _stub64
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
    mov rax, 0xFFFFFFFF
    and rdi, rax
    and rsi, rax
    and rdx, rax

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