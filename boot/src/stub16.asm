; magnetite_os - boot/src/stub16.asm
; A real mode stub for the second stage loader
;
; If a CPU is older than the i386, then
; clearly we've gone too far back in time.
;
; The tasks are not trivial, but well-defined:
; - Map out memory
; - Enable A20 gate
; - Load 32-bit GDT
; - Enter 32-bit protected mode
;
; TODO: implement CPU generation checks
;
; Refer to 'boot/src/defs.asm' for memory layout

; (definitions included by 'boot/src/stub32.asm')

; Do not use absolute positioning, as this
; file will be included in 'stub32.asm'
[bits 16]

; --- Main routine --- ;
section .stub16
_stub16:
    ; Header (like, c'mon?)
    ; 0a. 16-bit near jump (e9 RR RR)
    ; 0b. NOP padding (90 90 90 90 90)
    ; 1.  ZX 32-bit offset to '_start' (VV VV VV VV 00 00 00 00)
    ; 2.  maybe-pointer to HAL vector table (here: NULL)
    ; 3.  NOP padding (90 90 90 90 90 90 90 90)
    jmp dword .start
    align 8, nop
.handover_offset:
    dd _start_offset, 0
.vt_offset:
    dd NULL, NULL
.pad:
    align 16, nop
.start:
    cli                                     ; FA - Kill interrupts
    xchg bx, bx                             ; 87 DB - Bochs breakpoint

    ; Zero segment registers
    xor bx, bx
    mov ds, bx
    mov es, bx

    ; Initialize stack
    ; - hopefully we have more than enough space
    ; and that the stack actually moves downwards.
    ; - set a 256 B buffer between bottom of stack and
    ; boot sector start
    mov ss, bx
    mov sp, 0x7b00

    ; Enforce flat addressing
    jmp 0:.vec
.vec:
    sti                                     ; Restore interrupts
    mov byte [bootdev], al                  ; Store boot drive number

; Scan for memory using E820
e820_scan:
    xchg bx, bx                             ; Breakpoint
    push edi                                ; Save EDI
.lma:
    ; Perform LMA check (wiki.osdev.org)
    clc                                     ; Clear carry flag
    int 0x12                                ; Check LMA size using BIOS
    jc panic                                ; Do not proceed if memory
                                            ; size cannot be assessed

    cmp ax, 128                             ; Make sure we have at least 
                                            ; 128 kB of continuous memory

    jl panic                                ; Do not proceed if LMA is
                                            ; smaller than 128 kB

    lea edi, [ADDR_E820_MAP + 16]           ; Store map at ADDR_E820_MAP + 16
    xor esi, esi                            ; Zero entry count
    xor ebx, ebx                            ; Zero EBX
.seek:
    xchg bx, bx                             ; Breakpoint
    cmp esi, E820_ENTRIES                   ; Do not proceed beyond
                                            ; this many entries
    jge .end                                ; - skip if true

    push edi                                ; Save EDI
    mov eax, 0xe820                         ; Set call number
    mov edx, 0x534D4150                     ; String: 'SMAP'
    mov ecx, 24                             ; Ask for 24-byte entries
    int 0x15                                ; Call BIOS
    jc .cleanup                             ; We're done already...
    cmp eax, 0x534D4150                     ; String: 'SMAP'
    jne .cleanup                            ; We're also done already...

    pop edi                                 ; Restore EDI
    add edi, 24                             ; Move EDI to the next 24-byte slot
    inc esi                                 ; Increase entry count
    test ebx, ebx                           ; Test completion flag
    jnz .seek                               ; Continue if not complete
    ; --- fall-through --- ;
.cleanup:
    pop edi
    jmp .end
.end:
    ; Store zero-extended base and entry count
    ; - If entry counts were to exceed
    ; 2**32 - 1 (which shouldn't happen),
    ; then something's already wrong, and
    ; missing other areas wouldn't be the
    ; worst of our problems
    lea ebx, [ADDR_E820_MAP + 16]           ; Calculate array base
    mov [ADDR_E820_MAP], ebx                ; Store array base
    xor ebx, ebx
    mov [ADDR_E820_MAP + 4], ebx            ; Zero-extend

    mov [ADDR_E820_MAP + 8], esi            ; Store entry count
    xor esi, esi
    mov [ADDR_E820_MAP + 12], esi           ; Zero-extend

    pop edi                                 ; Restore EDI

; ---- TODO ---- ;
; In future designs, memory layout scanning,
; initialization and 32-bit mode operation
; may be contained in a separate file loaded
; from a FAT16 volume.
; --- [TODO] --- ;

; Enable A20 gate - fast
fast_a20:
    xchg bx, bx
    in al, 0x92                             ; Read from port 0x92
    test al, 2                              ; Check if A20 is already enabled
    jnz .fast_a20_after
    or al, 2                                ; Set A20 bit
    and al, 0xfe                            ; Clear fast reset bit
    out 0x92, al                            ; Write to port 0x92
    pause                                   ; Pause (or 'rep nop' in older CPUs)
    jmp fast_a20                            ; Verify that the A20 get is set
.fast_a20_after:
    xchg bx, bx                             ; Breakpoint in Bochs 

; Enter 32-bit protected mode
enter_pm:
    ; Here we go...
    cli                                     ; Kill interrupts
    lgdt [gdt32.pointer]                    ; Load GDT address
    mov eax, cr0                            ; Read control register 0
    or eax, 1                               ; Set PE bit 
    mov cr0, eax                            ; Write back to CR0

    ; Perform far jump to segment 0x08 (described in the GDT)
    jmp gdt32.kern_cs:_stub32

    ; --- wishfull fall-through --- ;
panic:
    xchg bx, bx                              ; Breakpoint in Bochs

    ; Write error string to screen
    mov ah, 0x0e                            ; BIOS teletype function
    mov si, errmsg                          ; Point to error message
    sti                                     ; Enable all interrupts

.cont:
    lodsb                                   ; Read 1 byte from SI, then shift
    cmp al, 0                               ; End of string (zero-terminated)
    je .done
    int 0x10                                ; Call BIOS
    jmp .cont                               ; Resume loop
.done:
    xor ax, ax                              ; Clear AX
    int 0x16                                ; Wait for keystroke
    int 0x19                                ; Reboot system

    ; If all else fails, force a triple fault
    ; - use 0x7b00 as source
    lidt [0x7b00]
    int 0x00

; Variables
errmsg      db "Boot failed. Replace boot device and reset.", 0
