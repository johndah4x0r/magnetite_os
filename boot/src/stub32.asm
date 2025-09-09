; magnetite_os - boot/src/stub32.asm
; A protected mode stub for the second stage loader
;
; The tasks are not trivial, but are well-defined:
; - check for long mode capability
; - initialize PAE-style paging
; - enable PAE
; - enable paging
; - load 64-bit GDT
; - execute embedded 64-bit loader
;
; Refer to 'boot/src/defs.asm' for memory layout

; Include definitions
%include "boot/src/defs.asm"

; Define external labels
extern _stub64                      ; Wrapped 64-bit code label

; Include 16-bit stub
%include "boot/src/stub16.asm"

; Do not use absolute positioning, as the
; binaries will be linked at a later stage
[bits 32]
section .text
_stub32:
    ; Kill interrupts (if they are still active)
    cli
    xchg bx, bx                             ; Breakpoint
    mov eax, gdt32.kern_ds                  ; Point to kernel data segment

    ; - Set data segments
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    ; - reset stack
    mov ss, ax                              ; Set stack segment register
    mov esp, 0x7b00                         ; Reset stack pointer

    ; Load defined quantities into memory
    ; (for compatibility reasons)
    mov eax, ADDR_E820_MAP
    mov ebx, OEM_LABEL

    ; Store them locally
    mov [e820_map.low], eax     ; E820 map location
    mov [oem_label.low], ebx    ; BPB location

; Check if long mode is supported
; - CPUID is assumed to be supported,
; as its existence should have been
; tested by brute force
check_lm:
    ; Check if CPUID supports extended features
    mov eax, EXT_CPUID          ; Check highest EAX parameter
    cpuid
    cmp eax, FEAT_CPUID         ; If EAX < FEAT_CPUID, no dice...
    jb .no_lm

    ; Check if long mode is supported
    mov eax, FEAT_CPUID         ; Check for features
    cpuid
    test edx, LM_EDX_CPUID      ; Check if the LM bit is set
    jnz .end                    ; Exit if it is set
    ; --- fall-through --- ;
.no_lm:
    ; Panic if long mode is not supported
    lea esi, [msgs.no_lm]       ; Load pointer to reason
    jmp panicb                  ; Panic - never to return...
.end:

; Disable 32-bit paging
; (unlikely that paging is up, since
; we control the environment)
disable_paging32:
    mov eax, cr0                ; Load CR0 into EAX
    and eax, NO_PAGING          ; Unset paging bit
    mov cr0, eax                ; Store modified CR0 

; Initialize 64-bit paging
; - essentially the same thought process
;   as the one outlined in 'wiki.osdev.org'
; TODO: maybe consider higher-half mapping
;       in addition to identity mapping
init_paging:
    xchg bx, bx                 ; Breakpoint

    ; Clear the master page hierarchy
    lea edi, [PML4T_ADDR]       ; Point EDI to the highest table
    mov cr3, edi                ; Let the CPU know where the tables are

    ; Write 4 * 1kiB, which should cover
    ; all four hierarchy levels
    xor eax, eax
    mov ecx, SIZEOF_PT
    rep stosd

    xchg bx, bx                 ; Breakpoint
    mov edi, cr3                ; Reset EDI back to the highest table

.set_flags:
    ; Set flags for each level
    ; - EDI is equal to PML4 address
    mov dword [edi], PDPT_ADDR & PT_ADDR_MASK | PT_PRESENT | PT_READWRITE

    mov edi, PDPT_ADDR
    mov dword [edi], PDT_ADDR & PT_ADDR_MASK | PT_PRESENT | PT_READWRITE

    mov edi, PDT_ADDR
    mov dword [edi], PT_ADDR & PT_ADDR_MASK | PT_PRESENT | PT_READWRITE

.fill_pt:
    ; Populate PT 0 to identity-map 0-2 MiB
    ; - which means using standard pages
    lea edi, [PT_ADDR]          ; Point EDI to PT 0

    ; - set flags
    ; - map PT 0, page 0 to 0-4 kiB
    mov eax, PT_PRESENT | PT_READWRITE
    mov ecx, ENTRIES_PER_PT     ; Fill PT 0

.set_entry_ident:
    mov dword [edi], eax        ; Write entry to [EDI]
    add eax, SIZEOF_PAGE        ; Map next physical page
    add edi, SIZEOF_PT_ENTRY    ; Write to next entry
    loop .set_entry_ident
.end:
    xchg bx, bx                 ; Breakpoint

.enable_pae:
    mov eax, cr4                ; Load CR4 into EAX
    or eax, PAE_ENABLE          ; Enable PAE in CR4
    mov cr4, eax                ; Store modified CR4

; Enable long mode and hand over
; control to 64-bit code
; 
; The contract is as follows:
; - EDI: pointer to OEM label pointer
; - ESI: pointer to boot drive number
; - EDX: pointer to E820 map pointer
; - ECX: zero-extended data segment number
enable_lm:
    ; Enable IA-32e mode first
    mov ecx, EFER_MSR
    rdmsr
    or eax, EFER_LME
    wrmsr

    ; Then enable paging
    mov eax, cr0
    or eax, PG_ENABLE
    mov cr0, eax

    ; Load arguments according to
    ; the System V AMD64 ABI
    ; - this isn't strictly needed,
    ;   but it would make the
    ;   handover to 'main' drastically
    ;   easier
    ; - nor are we required to follow
    ;   the convention strictly, as we are
    ;   handing over control to 64-bit code
    lea edi, [oem_label]        ; Point EDI to OEM label pointer
    lea esi, [bootdev]          ; Point ESI to boot drive number pointer
    lea edx, [e820_map]         ; Point EDX to E820 map pointer

    ; Load data segment number to CX
    ; - very important to NOT modify
    ;   pre-handover
    xor ecx, ecx
    mov cx, gdt64.data

    ; Load 64-bit GDT and perform far jump
    ; - for now, use identity mapping
    lgdt [gdt64.pointer]
    jmp gdt64.code:_stub64

    ; Halt in the event of a failure
    ; (unreachable)
    cli
    hlt

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
section .data
; 32-bit GDT
gdt32:
    ; Null descriptor (0x00)
    .null:
        dq 0                                ; 4 x 16 zeroes
    ; Kernel mode descriptor (0x08, 0x10)
    .kern_cs: equ $ - gdt32
        .kern_cs.limit_l    dw 0xffff       ; Limit         (00-15)
        .kern_cs.base_l     dw 0x0000       ; Base          (16-31)
        .kern_cs.base_m     db 0x00         ; Base          (32-39)
        .kern_cs.access     db 0x9a         ; Access        (40-47)
        .kern_cs.lim_h_fl   db 0xcf         ; Limit + flags (48-55)
        .kern_cs.base_h     db 0x00         ; Base          (56-63)
    .kern_ds: equ $ - gdt32
        .kern_ds.limit_l    dw 0xffff       ; Limit         (00-15)
        .kern_ds.base_l     dw 0x0000       ; Base          (16-31)
        .kern_ds.base_m     db 0x00         ; Base          (32-39)
        .kern_ds.access     db 0x92         ; Access        (40-47)
        .kern_ds.lim_h_fl   db 0xcf         ; Limit + flags (48-55)
        .kern_ds.base_h     db 0x00         ; Base          (56-63)
    .pointer:
        dw $ - gdt32 - 1                    ; Size of GDT - 1
        dd gdt32                            ; Base of GDT

; 64-bit GDT
gdt64:
    .null: equ $ - gdt64
        dq 0
    .code: equ $ - gdt64
        .code.limit_lo   dw 0xffff
        .code.base_lo    dw 0x0000
        .code.base_mid   db 0x00
        .code.access     db SEG_PRESENT | SEG_NOT_SYS | SEG_EXEC | SEG_RW
        .code.sflags     db SEG_GRAN_4K | SEG_LONG_MODE | 0x0f
        .code.base_high  db 0x00
    .data: equ $ - gdt64
        .data.limit_lo   dw 0xffff
        .data.base_lo    dw 0x0000
        .data.base_mid   db 0x00
        .data.access     db SEG_PRESENT | SEG_NOT_SYS | SEG_RW
        .data.sflags     db SEG_GRAN_4K | SEG_SZ_32 | 0x0f
        .data.base_high  db 0x00
    .pointer:
        dw $ - gdt64 - 1
        dd gdt64, 0

; Null-terminated messages
msgs:
    .panic16    db "Loader panicked while in 16-bit mode; CS:IP = ", 0
    .panic      db "Loader panicked while in 32-bit PM. Reason: ", 0
    .reason     db "Reason: ", 0
    .unsup      db "CPU older than i486, or otherwise unsupported", 0
    .no_lm      db "CPU does not support x86-64 long mode", 0
    .got_id     db "CPU vendor string: ", 0
    .caught_ud  db "Caught #UD at CS:IP = ", 0
    .e820       db "Failed to generate memory layout map", 0
    .reset      db "Replace boot device, and press <Enter> to reset", 0
    .crlf       db 0x0a, 0x0d, 0

; Nibble characters
numstr:
    .high       db "yyyy"
    .delim      db ":"
    .low        db "xxxx"
    .null       db 0
    .chrset     db "0123456789abcdef"

; Signature obtained from CPUID
signature:
    .low        dd 0
    .mid        dd 0
    .high       dd 0
    .null       db 0

; Pointer to BPB (32-bit pointer extended to 64-bit)
oem_label:
    .low        dd 0
    .high       dd 0

; Boot device (8-bit extended to 64-bit)
bootdev:
    .low        dd 0
    .high       dd 0

; Pointer to E820 map (32-bit pointer extended to 64-bit)
e820_map:
    .low        dd 0
    .high       dd 0