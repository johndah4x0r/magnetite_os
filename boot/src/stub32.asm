[bits 32]

; Do not use absolute positioning, as the
; binaries are linked at a later stage

extern _start                       ; Wrapped 64-bit code label
extern _start_offset                ; Offset to _start (defined by linker)

NULL            equ 0               ; Null pointer

; Flags for later use
ID_EFLAGS       equ 1 << 21         ; EFLAGS ID bit
EXT_CPUID       equ 1 << 31         ; CPUID extensions
FEAT_CPUID      equ (1 << 31) | 1   ; CPUID extended features
LM_EDX_CPUID    equ 1 << 29         ; Long mode bit
NO_PAGING       equ 0x7fffffff      ; No 32-bit paging

PAE_ENABLE      equ 1 << 5          ; Enable PAE in CR4
PG_ENABLE       equ 1 << 31         ; Enable paging in CR0


; Page hierarchy layout
PML4T_ADDR      equ 0x1000          ; Location of PML4 table (master hierarchy)
PDPT_ADDR       equ 0x2000          ; Location of PDP table (huge)
PDT_ADDR        equ 0x3000          ; Location of page directory table (large)
PT_ADDR         equ 0x4000          ; Location of page table (standard)


; Page masks and flags
PT_ADDR_MASK    equ 0xffffffffff000 ; Mask to align addresses to 4 kiB
PT_PRESENT      equ 1               ; Marks page as present
PT_READWRITE    equ 2               ; Marks page as R/W
PT_PAGESIZE     equ 128             ; Marks page as large/huge (if needed)

SIZEOF_PAGE     equ 1 << 12         ; Sets page size to 4 kiB (normal pages)
SIZEOF_PT       equ 1 << 12         ; Sets page table size to 4 kiB

ENTRIES_PER_PT  equ 512             ; Entries per page table
SIZEOF_PT_ENTRY equ 8               ; Size of PT entry (64 bits)

EFER_MSR        equ 0xC0000080      ; EFER MSR address
EFER_LME        equ 0x100           ; EFER IA-32e set bit


; GDT bits

; - access bits
SEG_PRESENT        equ 1 << 7
SEG_NOT_SYS        equ 1 << 4
SEG_EXEC           equ 1 << 3
SEG_RW             equ 1 << 1

; - flags bits
SEG_GRAN_4K       equ 1 << 7
SEG_SZ_32         equ 1 << 6
SEG_LONG_MODE     equ 1 << 5

NULL            equ 0
section .text
_stub32:
    ; Header (like, c'mon?)
    ; 0a. (5) 32-bit near jump (e9 RR RR RR RR)
    ; 0b. (4) zero extension (00 00 00 00)
    ; 1.  (3) zero padding (00 00 00)
    ; 2.  (8) ZX 32-bit offset to '_start' (VV VV VV VV 00 00 00 00)
    jmp dword .start
.pad:
    times 12-($-$$) db 0
.handover_offset:
    dd _start_offset, NULL
.start:
    ; Kill interrupts (if they are still active)
    cli

    xchg bx, bx                 ; Breakpoint

    ; Unpack arguments stored in stack
    add esp, 4                  ; Ignore return address, as we
                                ; made the MBR call this loader,
                                ; rather than jump to it.

    pop eax                     ; pop E820 map location
    pop ebx                     ; pop BPB location
    pop ecx                     ; pop boot device number

    ; Store them locally
    mov [e820_map.low], eax     ; E820 map location
    mov [oem_label.low], ebx    ; BPB location
    mov [bootdev.low], ecx      ; boot device number

    ; Reset stack to 0x7b00
    mov esp, 0x7b00

; Check if CPUID is present
check_cpuid:
    pushfd                      ; Copy EFLAGS into stack
    pop eax                     ; Store EFLAGS into EAX

    mov ecx, eax                ; Store copy of EFLAGS

    ; Attempt to flip the ID bit in EFLAGS
    xor eax, ID_EFLAGS          ; Flip bit 21
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

; Check if long mode is supported
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
; *we* control the MBR source)
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

    ; Load data segment number to AX
    xor eax, eax
    mov ax, gdt64.data

    ; Load 64-bit GDT and perform far jump
    ; - for now, use identity mapping
    lgdt [gdt64.pointer]
    jmp gdt64.code:_start

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
align 16

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
    .panic      db "Loader panicked while in 32-bit PM. Reason: ", 0
    .no_cpuid   db "CPU appears to not support CPUID", 0
    .no_lm      db "CPU appears to not support x86-64 long mode", 0

; Pointer to BPB (32-bit pointer extended to 64-bit)
oem_label:
    .low        dd 0
    .high       dd 0

; Boot device (8-bit extended to 64-bit)
bootdev:
    .low        dd 0
    .high       dd 0

e820_map:
    .low        dd 0
    .high       dd 0