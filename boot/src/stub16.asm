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
; Refer to 'boot/src/defs.asm' for memory layout

; (definitions included by 'boot/src/stub32.asm')

; Do not use absolute positioning, as this
; file will be included in 'stub32.asm'
[bits 16]

; --- Main routine --- ;
section .stub16
_stub16:
    ; Header (like, c'mon?)
    ; 0a. (3) 16-bit near jump (e9 RR RR)
    ; 0b. (6) zero-extension (00 00 00 00 00 00)
    ; 1.  (3) zero padding (00 00 00)
    ; 2.  (8) ZX 32-bit offset to '_start' (VV VV VV VV 00 00 00 00)
    jmp word .start
.pad:
    times 12-($-$$) db 0
.handover_offset:
    dd _stub64 - _stub16, 0                 ; (offset provided by 'stub32.asm')
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
    cld                                     ; Clear DF
    mov byte [bootdev], al                  ; Store boot drive number

; Check for CPU capability
; - assumes IF = 0 for atomicity
cpu_check:
    ; Perform legal checks
    ; (0) check for changes in high FLAGS
    pushf                                   ; (2) Obtain original copy
    pop dx                                  ; (1) (store in DX)

    mov bx, dx                              ; (1) Obtain main copy
    xor bh, 0x70                            ; (1) Toggle high bits
    push bx                                 ; (2) Write modified FLAGS
    popf                                    ; (1) (pop it back)

    pushf                                   ; (2) Read back from FLAGS
    pop bx                                  ; (1) Store it in BX

    push dx                                 ; (1) Load original copy
    popf                                    ; (0) Restore original state

    and bh, 0x70                            ; Mask top nibble
    and dh, 0x70                            ; Mask top nibble
    cmp bh, dh                              ; Check whether the top bits have changed
    jne .illegal                            ; Continue if change is detected
    ; --- fall-through --- ;

    mov si, msgs.unsup
    call panic

.illegal:
    ; Load custom interrupt vector
    ; - only makes sense in CPUs newer
    ; than the 8086
    ; - use `xchg` for external inspection
    xor ax, ax                              ; Zero AX
    lea si, [ud_handler]                    ; Point SI to custom handler
    xchg [0x0018], si                       ; Load custom IP
    xchg [0x001a], ax                       ; Load custom CS

    ; Run a slew of illegal instructions to
    ; enforce minimum CPU capability
    ; - checks if `cpuid` works as intended,
    ; as it raises a #UD on older CPUs
    db 0x66                                 ; (operand size override)
    pusha                                   ; Push all 32-bit GPRs

    xor eax, eax                            ; Zero EAX (prefix implicit)
    cpuid                                   ; Identify CPU

    ; Store CPU authenticity string
    mov [signature.low], ebx
    mov [signature.mid], edx
    mov [signature.high], ecx

    db 0x66                                 ; (operand size override)
    popa                                    ; Pop all 32-bit GPRs

    ; Print vendor string to screen
    mov bp, sp                              ; Store old SP in BP

    ; - push messgaes in reverse order
    push msgs.crlf                          ; Newline    
    push signature                          ; Vendor string
    push msgs.got_id                        ; Preamble
    call print
.done:
    sti                                     ; Restore interrupts
    ; TODO

; Scan for memory using E820
e820_scan:
    xchg bx, bx                             ; Breakpoint

    ; Perform LMA check (wiki.osdev.org)
    clc                                     ; Clear carry flag
    xor ax, ax                              ; Zero AX
    int 0x12                                ; Check LMA size using BIOS
    jc .stop                                ; Do not proceed if memory
                                            ; size cannot be assessed

    cmp ax, 128                             ; Make sure we have at least 
                                            ; 128 kB of continuous memory

    jge .cont                               ; Proceed if true
    ; --- fall-through --- ;
.stop:
    mov si, msgs.e820                       ; Point SI to reason message
    call panic                              ; Call panic routine
.cont:
    lea edi, [ADDR_E820_MAP + 16]           ; Store map at ADDR_E820_MAP + 16
    xor esi, esi                            ; Zero entry count
    xor ebx, ebx                            ; Zero EBX
.seek:
    xchg bx, bx                             ; Breakpoint
    cmp esi, E820_ENTRIES                   ; Do not proceed beyond
                                            ; this many entries
    jge .end                                ; - skip if true

    mov dword [edi + 20], 1                 ; Force valid ACPI 3.x entry

    mov eax, 0xe820                         ; Set call number
    mov edx, 0x534d4150                     ; String: 'SMAP' (big-endian)
    mov ecx, 24                             ; Ask for 24-byte entries
    int 0x15                                ; Call BIOS
    jc .verify                              ; We're done already...
    cmp eax, 0x534d4150                     ; String: 'SMAP' (big-endian)
    jne .verify                             ; We're also done already...

    add edi, 24                             ; Move EDI to the next 24-byte slot
    inc esi                                 ; Increase entry count
    test ebx, ebx                           ; Test completion flag
    jnz .seek                               ; Continue if not complete
    ; --- fall-through --- ;
.verify:
    test esi, esi                           ; Check if we're at the first invocation
    jz .stop                                ; Panic if error occured on the first try
.end:
    ; Store zero-extended base and entry count
    ; - If entry count was to exceed
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

; Print error message and reset
; - SI: pointer to reason string
panic:
    xchg bx, bx                             ; Breakpoint in Bochs

    pop ax                                  ; Pop calling IP to AX
    mov dx, cs                              ; Store CS in DX
    call stringify_num                      ; Stringify CS:IP

    ; Write error string to screen
    mov bp, sp                              ; Store old value of SP

    ; - push messages in reverse order
    push msgs.reset                         ; Request to reset
    push msgs.crlf                          ; Newline
    push si                                 ; Provided reason
    push msgs.reason                        ; Prelude
    push msgs.crlf                          ; Newline
    push numstr                             ; CS:IP string
    push msgs.panic16                       ; Error message
    push msgs.crlf                          ; Newline

    call print                              ; Write pushed strings

    sti                                     ; Enable all interrupts
    xor ax, ax                              ; Clear AX
    int 0x16                                ; Wait for keystroke
    int 0x19                                ; Reboot system
    ; --- fall-through --- ;

    ; Provoke a triple fault
    ; - only works post-286
    lidt [0x7b00]
    int 0x00

; Exception handler for #UD (INT 0x06)
ud_handler:
    xchg bx, bx                             ; Breakpoint

    pop ax                                  ; Pop issuer IP
    pop dx                                  ; Pop issuer CS
    call stringify_num                      ; Stringify CS:IP

    ; Load messages in reverse order
    mov bp, sp
    push msgs.crlf                          ; Newline
    push numstr                             ; CS:IP string
    push msgs.caught_ud                     ; Error message
    push msgs.crlf                          ; Newline

    call print                              ; Write pushed strings

    xor bx, bx                              ; Zero BX
    mov ax, not_supported                   ; Load custom vector
    push bx                                 ; Push CS = 0
    push ax                                 ; Push custom vector
    iret                                    ; Return from interrupt
not_supported:
    mov si, msgs.unsup                      ; Load error message
    call panic                              ; Panic

; Print message pointed by stack arguments
; - takes BP (pre-push SP)
; - clobbers AX, CX, DX
print:
    pop dx                                  ; Pop return IP
    mov cx, bp                              ; Calculate entry count
    sub cx, sp                              ; (1)
    shr cx, 1                               ; (2)
    mov ah, 0x0e                            ; BIOS teletype function
.cont:
    pop si                                  ; Pop entry
.inner:
    lodsb                                   ; Read 1 byte from [DS:SI], then shift
    test al, al                             ; End of string (null-terminated)
    jz .done

    ; This part is admittedly inefficient,
    ; but it has to be if we don't want
    ; to affect FLAGS set by the caller
    pushf                                   ; Save FLAGS
    sti                                     ; Enable interrupts
    int 0x10                                ; Call BIOS
    popf                                    ; Restore FLAGS
    jmp .inner                              ; Resume loop
.done:
    loop .cont                              ; Parse more entries
    push dx                                 ; Push return IP
    ret                                     ; Return on exhaustion

; Convert DX:AX to 8-digit hex string
; - we'll use DX:AX for redundancy
stringify_num:
    pusha                                   ; Preserve GPRs
    pushf                                   ; Preserve FLAGS
    mov bx, ax                              ; Stringify AX
    mov di, numstr.low                      ; Point to low string
    call .conv

    mov bx, dx                              ; Stringify DX
    mov di, numstr.high                     ; Point to high string
    call .conv

    jmp .done
.conv:
    mov cx, 4
.top:
    ; Endianness is most certainly a
    ; fun thing to deal with...
    push bx                                 ; Save BX
    shr bx, 12                              ; Capture highest nibble
    lea si, [numstr.chrset + bx]            ; Obtain character
    pop bx                                  ; Restore BX
    movsb                                   ; Write to buffer (increment DI)
    shl bx, 4                               ; Discard highest nibble
    loop .top                               ; Repeat loop (decrement CX)
    ret
.done:
    popf                                    ; Restore FLAGS
    popa                                    ; Restore GPRs
    ret
