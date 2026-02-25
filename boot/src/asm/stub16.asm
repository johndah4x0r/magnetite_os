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
    ; Header (16)
    ; 0a. (3) 16-bit near jump (e9 RR RR)
    ; 0b. (1) byte extension (90)
    ; 1.  (4) 32-bit size of stage-2 loader
    ; 2.  (4) 32-bit size of BSS region
    ; 3.  (4) 32-bit address to end of stage-2 loader space
    jmp word .start
    nop
.binsize:
    dd _sizeof_s2_ldr                       ; (size provided by linker)
.bss_size:
    dd _sizeof_bss                          ; (size provided by linker)
.region_end:
    dd 0                                    ; (address calculated at runtime)
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
    mov sp, INIT_STACK

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
    jne .is_newer                           ; Continue if change is detected
    ; --- fall-through --- ;

    mov si, msgs.unsup
    call panic
.is_newer:
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
    xchg bx, bx                             ; Breakpoint

; Initialize BSS
init_bss:
    push es                                 ; Save ES
    lea eax, [_stub16]                      ; Obtain start of stage-2 loader space
    mov ecx, [_stub16.binsize]              ; Obtain size of loader binary
    add eax, ecx                            ; Calculate start of BSS

    ; Calculate end of stage-2 loader space
    mov edx, eax                            ; Copy EAX (start of BSS) to EDX
    add edx, [_stub16.bss_size]             ; Increment start of BSS by BSS size 
    mov [_stub16.region_end], edx           ; Store it as end of stage-2 loader space

    ; Calculate segment:offset representation
    ; of the address in EAX
    mov ebx, 16                             ; Set EBX = 16
    xor edx, edx                            ; Zero EDX
    div ebx                                 ; Divide 0:EAX by EBX = 16
    mov es, ax                              ; Set ES = AX (segment)
    mov di, dx                              ; Set DI = DX (offset)
.top:
    ; Do not perform any more initialization if
    ; the BSS size is equal to zero
    test ecx, ecx
    jz .done

    xor eax, eax                            ; Zero EAX
    test ecx, 0x0003                        ; Check whether ECX is a multiple of 4
    jz .copy_quad                           ; If it is, copy four bytes 
    ; --- fall-through --- ;

    mov byte es:[di], al                    ; Copy a single byte
    dec ecx                                 ; Decrement ECX by one
    add di, 1                               ; Increment DI by 1
    jc .recalc                              ; Recalculate ES:DI on overflow
.copy_quad:
    mov dword es:[di], eax                  ; Copy four bytes
    sub ecx, 4                              ; Decrement ECX by four
    add di, 4                               ; Increment DI by 1
    jnc .top                                ; Go back to top of loop if CF = 0
.recalc:
    ; Try to change ES "atomically"
    ; TODO: maybe handle CF after `add bx, 0x1000`?
    mov bx, es                              ; Copy ES into BX
    add bx, 0x1000                          ; Increment BX = ES by 4096
    mov es, bx                              ; Copy new BX into ES
    jmp .top                                ; Go back to top of loop
.done:
    pop es                                  ; Restore ES
 
; Set video mode
set_video_mode:
    pushfd                                  ; Preserve flags

    ; Obtain information about the VBIOS
    push es                                 ; Preserve ES
    lea di, [vbe_info_block]                ; Point DE to VBE info block
    mov ax, 0x4f00
    clc                                     ; Clear CF
    int 0x10                                ; Call BIOS
    pop es                                  ; Restore ES

    cmp ax, 0x004f                          ; Check whether the call was a success
    jne .default                            ; If not, we're done here

    mov eax, [vbe_info_block.signature]     ; Obtain signature
    cmp eax, 0x41534556                     ; String: "VESA" (little-endian)

    ; Point DS:SI to the video modes array
    mov dx, [vbe_info_block.video_mode_segment]
    mov ds, dx
    mov si, [vbe_info_block.video_mode_offset]

    cld                                     ; Clear DF
    xor ax, ax                              ; Zero AX
.top:
    lodsw                                   ; Load one word from the array
    cmp ax, 0xFFFF                          ; Check if we're at the end of the array
    je .more                                ; If we are, break the loop

    ; Process provided mode
    ; TODO: do something useful
    push si                                 ; Preserve SI
    xor dx, dx                              ; Zero DX
    call stringify_num                      ; Stringify 0000:AX

    mov bp, sp                              ; Store pre-push SP
    push msgs.crlf                          ; Print newline *last*
    push numstr                             ; Print stringified number
    push msgs.vbe_mode                      ; Print prelude
    call print

    pop si                                  ; Restore SI
    jmp .top                                ; Restart loop
.more:
    ; -- TODO -- ;
    xchg bx, bx                             ; Breakpoint
.default:
    push es                                 ; Preserve ES
    mov ax, 0x0003                          ; Set video mode to 0x03 (VGA text mode)
    clc                                     ; Clear CF
    int 0x10                                ; Call BIOS
    pop es                                  ; Restore ES
.done:
    popfd                                   ; Restore flags
    xchg bx, bx                             ; Breakpoint

; Scan for memory using E820
e820_scan:
    mov eax, [_stub16.region_end]           ; Obtain end of stage-2 loader space

    ; Align to nearest 16 bytes
    test ax, 0x000f                         ; Check the lowest nibble
    jz .aligned                             ; Skip alignment if already aligned
    and ax, 0xfff0                          ; Discard lowest nibble
    add ax, 0x0010                          ; Increment nibble (basically apply `ceil(ax)`)
.aligned:
    ; Print the location of the
    ; E820 map descriptor
    push eax                                ; Save EAX
    mov edx, eax                            ; Copy EAX to EDX
    shr edx, 16                             ; Set DX to the high 16 bits of EAX
    call stringify_num                      ; Stringify DX:AX

    mov bp, sp                              ; Save pre-push stack pointer
    push msgs.crlf
    push numstr
    push msgs.map_start
    call print                              ; Print composed message

    pop eax                                 ; Restore EAX
    mov [e820_map.low], eax                 ; store it as the location for the E820 map
.check_lma:
    ; Perform LMA check (wiki.osdev.org)
    clc                                     ; Clear carry flag
    xor ax, ax                              ; Zero AX
    int 0x12                                ; Check LMA size using BIOS
    jc .stop                                ; Do not proceed if memory
                                            ; size cannot be assessed

    cmp ax, 256                             ; Make sure we have at least 
                                            ; 256 kiB of continuous memory

    jge .cont                               ; Proceed if true
    ; --- fall-through --- ;
.mem_too_small:
    mov si, msgs.mem_too_small              ; Point SI to reason message
    call panic                              ; Call panic routine
.cont:
    ; Store map at `[e820_map.low] + E820_DESC_END`
    mov esi, [e820_map.low]
    lea edi, [esi + E820_DESC_END]          ; - wide address in EDI

    ; Calculate segment:offset equivalent
    ; of wide address in EDI, then store
    ; it in ES:DI
    mov edx, edi                            ; Copy wide address into EDX
    mov ax, di                              ; Copy lower-half into AX
    mov cx, 16                              ; Set CX = 16, as it can be used later
    shr edx, cl                             ; Make DX show the upper half of EDI
    ; (EDI is now in DX:AX form)

    ; Divide DX:AX by 16, so that
    ; - AX contains the quotient (segment)
    ; - DX contains the remainder (offset)
    div cx

    mov es, ax                              ; Set ES = AX
    mov di, dx                              ; Set DI = DX

    xor esi, esi                            ; Zero entry count
    xor ebx, ebx                            ; Zero EBX
.seek:
    xchg bx, bx                             ; Breakpoint
    cmp esi, E820_ENTRIES                   ; Do not generate more than `E820_ENTRIES`
    jge .end                                ; (skip if `ESI >= E820_ENTRIES`)

    ; Save current value of ES
    mov cx, es

    ; Save current value of ESI
    push esi

    ; Perform segmentation-friendly
    ; pointer advance
    xor dx, dx
    mov si, di                              ; Set SI = DI
    mov ax, es                              ; Set AX = ES
    add si, 20                              ; Advance SI by 20 bytes
    adc dx, 0                               ; Preserve carry
    jc .stop                                ; Stop on overflow
    shl dx, 12                              ; Calculate segment equivalent
    adc ax, dx                              ; Increment segment
    jc .stop                                ; Stop on overflow
    mov es, ax                              ; Update ES

    mov word es:[si], 0x0001                ; Force valid ACPI 3.x entry

    ; Perform segmentation-friendly
    ; pointer advance
    xor dx, dx
    add si, 2                               ; Advance SI by 2 bytes
    adc dx, 0                               ; Preserve carry
    jc .stop                                ; Stop on overflow
    shl dx, 12                              ; Calculate segment equivalent
    adc ax, dx                              ; Increment segment
    jc .stop                                ; Stop on overflow
    mov es, ax
    mov word es:[si], 0x0000

    ; Restore old value of ES
    mov es, cx

    ; Restore ESI
    pop esi

    ; Prepare call to E820
    ; - we assume that ES:DI has been
    ; correctly calculated in the last
    ; iteration
    mov eax, 0x0000e820                     ; Set call number
    mov edx, 0x534d4150                     ; String: 'SMAP' (big-endian)
    mov ecx, 24                             ; Ask for 24-byte entries
    clc                                     ; Clear residual CF
    int 0x15                                ; Call BIOS
    jc .verify                              ; We're done already...
    cmp eax, 0x534d4150                     ; String: 'SMAP' (big-endian)
    jne .verify                             ; We're also done already...

    ; Perform segmentation-friendly
    ; pointer advance
    xor dx, dx
    mov ax, es                              ; Set AX = ES
    add di, 24                              ; Advance DI by 24 bytes
    adc dx, 0                               ; Preserve carry
    jc .stop                                ; Stop on overflow
    shl dx, 12                              ; Calculate segment equivalent
    adc ax, dx                              ; Increment segment
    jc .stop                                ; Stop on overflow
    mov es, ax

    inc esi                                 ; Increase entry count
    test ebx, ebx                           ; Test completion flag
    jnz .seek                               ; Continue if not complete
    ; --- fall-through --- ;
.verify:
    test esi, esi                           ; Check if we're at the first invocation
    jnz .end                                ; Proceed if not
    ; --- fall-through --- ;
.stop:
    mov si, msgs.e820                       ; Point SI to reason message
    call panic                              ; Call panic routine
.end:
    xchg bx, bx

    ; Zero ES
    xor ax, ax
    mov es, ax

    ; Store zero-extended base and entry count
    ; - If entry count was to exceed
    ; 2**32 - 1 (which shouldn't happen),
    ; then something's already wrong, and
    ; missing other areas wouldn't be the
    ; worst of our problems
    mov eax, [e820_map.low]
    push esi
    mov esi, eax

    ; Divide 0:EAX by 16 to obtain segment and offset
    xor edx, edx                            ; Zero EDX
    mov ecx, 16                             ; Divide 0:EAX by 16
    div ecx                                 ; (here)
    ; (ignore high words of EAX and EDX)

    mov es, ax                              ; Set ES to quotient (segment)
    mov di, dx                              ; Set DI to remainder (offset)

    lea ebx, [esi + E820_DESC_END]          ; Calculate array base
    pop esi

    mov es:[di + E820_DESC_ADDR], ebx       ; Store array base
    xor ebx, ebx
    mov es:[di + E820_DESC_ADDR + 4], ebx   ; Zero-extend

    mov es:[di + E820_DESC_SIZE], esi       ; Store entry count
    xor esi, esi
    mov es:[di + E820_DESC_SIZE + 4], esi   ; Zero-extend

; Enable A20 gate - fast
fast_a20:
    xor cx, cx                              ; Zero CX (in case it gets consumed)
    mov dl, 0xff                            ; Only allow 255 iterations (do NOT use CX)
.fast_a20_top:
    xchg bx, bx                             ; Breakpoint
    in al, 0x92                             ; Read from port 0x92
    test al, 2                              ; Check if A20 is already enabled
    jnz .fast_a20_after                     ; Exit verification loop
    ; --- fall-through --- ;

    or al, 2                                ; Set A20 bit
    and al, 0xfe                            ; Clear fast reset bit
    out 0x92, al                            ; Write to port 0x92
    pause                                   ; Pause (or 'rep nop' in older CPUs; may consume CX)
    
    dec dl                                  ; Decrement DL
    test dl, dl                             ; Check if DL > 0
    jnz .fast_a20_top                       ; Verify that the A20 gate is set
    ; --- fall-through on exhaustion --- ;

    mov si, msgs.unsup                      ; Print error message (unsupported CPU)
    call panic                              ; Terminate gracefully
.fast_a20_after:
    xchg bx, bx                             ; Breakpoint in Bochs 

create_e820_desc:

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
    xor ax, ax
    mov ds, ax
    mov es, ax

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
; - clobbers AX, CX, DX, SI (very important!)
print:
    pop dx                                  ; Pop return IP
    mov cx, bp                              ; Calculate entry count
    sub cx, sp                              ; (1) calculate difference in bytes
    shr cx, 1                               ; (2) divide by two to obtain count
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

    ; Preserve segment registers
    push ds
    push es

    xor bx, bx                              ; Zero BX
    mov ds, bx                              ; Zero DS
    mov es, bx                              ; Zero ES

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
    ; Restore segment register
    pop es
    pop ds

    popf                                    ; Restore FLAGS
    popa                                    ; Restore GPRs
    ret
