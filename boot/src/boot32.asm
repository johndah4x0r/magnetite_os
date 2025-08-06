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
    ; Kill interrupts (if they are still active)
    cli

    ; Unpack arguments stored in stack
    add esp, 4                  ; Ignore return address, as we
                                ; made the MBR call this loader,
                                ; rather than jump to it.

    pop eax                     ; pop BPB location
    pop ebx                     ; pop active text mode
    pop ecx                     ; pop boot device number

    ; Store them locally
    mov [oem_label.low], eax    ; BPB location
    mov [textmode.low], ebx     ; active text mode
    mov [bootdev.low], ecx      ; boot device number

    ; Reset stack to 0x7b00
    mov esp, 0x7b00

; Check if CPUID is present
check_cpuid:
    pushfd                      ; Copy EFLAGS into stack
    pop eax                     ; Store EFLAGS into EAX

    mov ecx, eax                ; Store copy of EFLAGS

    ; Attempt to flip the ID bit in EFLAGS
    xor eax, 0x200000           ; Flip bit 21
    push eax                    ; Store modified EFLAGS
    popfd                       ; Copy from stack

    pushfd                      ; Copy EFLAGS into stack
    pop eax                     ; Store EFLAGS into EAX

    ; Restore EFLAGS to its original state
    push ecx                    ; Store original EFLAGS
    popfd

    ; Make sure that the ID bit is flipped
    cmp eax, ecx
    jne .end

    ; --- fall-through --- ;
    ; Panic if CPUID is in fact not present
    lea esi, [msgs.no_cpuid]    ; Load pointer to reason
    jmp panicb                  ; Panic - never to return...
.end:


; ---- Routines ---- ;

; Print line to screen
; - does NOT parse LF
; Accepts:
; - ESI: pointer to null-terminated string
; - DS, ES = 0x10: data segment
printb:
    ; Initialize VGA-compatible character
    xor ax, ax                  ; Zero AX
    mov ah, 0x0f                ; White-on-black

    ; Set target to 0xb8000
    mov edi, 0xb8000
.top:
    ; Load from [DS:ESI] into AL (thankfully)
    lodsb                       ; Load and auto-increment
    test al, al                 ; Break loop on null
    jz .end

    ; Write from AX into [ES:EDI]
    stosw                       ; Write character to buffer
    jmp .top                    ; Go back to top
.end:
    ret

; Panic with reason
; Accepts:
; - ESI: pointer to null-terminated string
; - DS, ES = 0x10: data segment
panicb:
    ; Print preamble first
    push esi                    ; Save original ESI
    lea esi, [msgs.panic]       ; Point ESI to preamble
    call printb

    ; Then print reason
    pop esi                     ; Restore original ESI
    call printb

    ; Halt indefinitely
    ; - ideally, we'd poll the keyboard and reset, 
    ; but that's not feasible nor necessary in a 
    ; 32-bit PM stub - which is a transient stage
    cli                         ; Kill interrupts
    hlt

; separating text from data isn't that
; crucial in binary executables, but
; it is best practice (we'll merge
; them anyways during linking)
section .data.preboot

; Null-terminated messages
msgs:
    .panic      db "Loader panicked while in 32-bit PM. Reason: ", 0
    .no_cpuid   db "CPU does not appear to expose CPUID", 0

; Pointer to BPB (32-bit pointer extended to 64-bit)
oem_label:
    .low        dd 0
    .high       dd 0

; Active text mode (16-bit extended to 64-bit)
textmode:
    .low    dd 0
    .high   dd 0

; Boot device (8-bit extended to 64-bit)
bootdev:
    .low    dd 0
    .high   dd 0