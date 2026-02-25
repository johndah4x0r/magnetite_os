[bits 64]

; Do not use absolute positioning, as the
; binaries are linked at a later stage
;
; This is especially important, as we want
; the secondary (tertiary?) stage to be
; independent of initial position.

; Include definitions
%include "boot/src/asm/defs.asm"

global _stub64          ; Global export of _stub64
extern _start           ; Import of Rust '_start' routine

NULL    equ 0           ; Null pointer

section .stub64
_stub64:
    ; Bye-bye, useless header!
    ; See you in rebase hell!
    xchg bx, bx

    ; Until we have a usable IDT, we should
    ; disable interupts before it's too late
    cli

    ; Initialize segments
    ; - assuming EAX is preserved mid-jump,
    ;   we should be able to see the data
    ;   segment number in ECX
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    ; Reset stack
    ; - this is the only occasion where
    ; we'd be using static references
    mov rsp, INIT_STACK
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
    ; - (RCX: absolute pointer to screen info struct)

    ; - limit access to 32-bit space
    mov rax, 0xFFFFFFFF
    and rdi, rax
    and rsi, rax
    and rdx, rax
    and rcx, rax

    mov rdi, [rdi]
    mov rsi, [rsi]
    mov rdx, [rdx]

    ; TODO
    ; Call Rust main routine
    ; - NEVER perform a far call, as there's
    ; no need to change segments, and we'd
    ; otherwise run the risk of feeding RIP 
    ; with blatantly incorrect values
    call _start
    ; --- fall-through (unlikely) --- ;

.spin:
    ; Freeze without eternally halting
    ; (to avoid killing debuggers...)
    pause               ; Signal spin loop
    jmp .spin           ; Repeat