; magnetite_os - boot/boot0.asm
;
; The *absolute* initial stage of every boot sequence
; you can find out there in the wild.
;
; Initialize the system, set the video mode, load
; second stage into 0x8000, enable protected mode
; and transfer control to second stage.
; 
; Boot parameters can be found in 'params' (0x7e00).
; - parameters are 16-bit values, which must
;   be aligned for FFI compatibility
;
; Low Memory is used as follows:
; * 0x00500-0x07af0 - stack (to be relocated)
; * 0x07b00-0x07bff - guard region (not enforced)
; * 0x07c00-0x07dff - MBR
; * 0x07e00-0x095ff - second-stage loader
; * 0x09600-0x097ff - guard region (not enforced)
; * 0x09800-0x0ffff - E820 memory map
; * 0x10000-0x1ffff - minimum usable memory (=128 kB)
; * 0x20000-0x7ffff - maximum usable memory (>128 kB)

[bits 16]
[org 0x7c00]

; We'll just assume that the very first bytes of the
; target MBR are 'EB 3C 90' and work from there.

jmp short _start
nop

; - Dummy BIOS parameter block (DOS 4.0)
; Used for references, skipped when overwriting target MBR
; ...doesn't stop me from decorating it, though...
OemLabel:
    db "MGNTTEOS"
BytesPerSector:
    dw 512
SectorsPerCluster:
    db 4
ReservedSectors:
    dw 12
FatCount:
    db 2
RootDirEntries:
    dw 512
SectorsCount:
    dw 0
MediumType:
    db 0xf8
SectorsPerTab:
    dw 64
SectorsPerTrack:
    dw 16
Heads:
    dw 8
HiddenSectors:
    dd 0
LargeSectors:
    dd 65536
DriveNumber:
    dw 0
Signature:
    db 0x29
VolumeId:
    dd 0x1337c0de
VolumeLabel:
    db "MAGNETITEOS"
FileSystem:
    db "FAT16   "

; --- Main routine --- ;
_start:
    ; now what do I do?
    ; Zero out segment registers 
    ; and initialize stack
    ; The first 3 instructions also serve as landmarks
    ; to determine where the BPB ends and where bootstrap
    ; code starts.
    cli                                     ; FA - Kill interrupts
    xchg bx, bx                             ; 87 DB - Bochs breakpoint
    xor bx, bx                              ; 31 DB - Zero BX
    mov ds, bx
    mov es, bx

    ; Initialize stack
    ; - hopefully we have more than enough space
    ; and that the stack actually moves downwards.
    ; - set a 256 B buffer between bottom of stack and
    ; boot sector start
    mov ss, bx
    mov sp, 0x7b00

    ; - Store device number here - ;
    cmp dl, 0
    je .next

    ; Find storage device geometry
    mov [bootdev], dl
    mov ah, 8
    int 0x13
    jc panic
    and cx, 0x3f
    mov [SectorsPerTrack], cx
    movzx dx, dh
    inc dx
    mov [Heads], dx
    ; - fall-through - ;
.next:

; Set video mode - now
set_vid:
    ; Try to set video mode to EGA 80x43 (0x17)
    mov ax, 0x0017                          ; AH=0x00, AL=0x17 - set video mode to 0x17
    int 0x10                                ; Call BIOS

    ; Confirm video mode
    mov ax, 0x0f00                          ; AH = 0x0F, AL=* - get current video mode
    int 0x10                                ; Call BIOS
    cmp al, 0x17                            ; Check whether the video mode was successfully set
    je .is_43                               ; If so, skip ahead
    ; - fall-through on failure - ;

    ; Reset video mode to 80x25 (0x03)
    mov ax, 0x0003                          ; AH=0x00, AL=0x03 - set video mode to 0x03
    int 0x10                                ; Call BIOS
    mov ax, 0x0f00                          ; AH=0x0F, AL=* - get current video mode
    int 0x10                                ; Call BIOS
    cmp al, 0x03                            ; Check whether the video mode was successfully set
    jne panic                               ; If not, panic with whatever mode we have
    ; - fall-through on success - ;

.is_25:
    mov ah, 25                              ; 25 rows
    jmp .end
.is_43:
    mov ah, 43                              ; 43 rows
.end:
    mov al, 80                              ; 80 columns
    mov [textmode], ax                      ; Store text mode

; Scan for memory using E820
e820_scan:
    xchg bx, bx                             ; Breakpoint
    push edi                                ; Save EDI
.lma:
    clc                                     ; Clear carry flag
    int 0x12                                ; Check LMA size using BIOS
    jc panic                                ; Do not proceed if memory
                                            ; size cannot be assessed

    cmp ax, 128                             ; Make sure we have at least 
                                            ; 128 kB of continuous memory

    jl panic                                ; Do not proceed if LMA is
                                            ; smaller than 128 kB

    lea edi, [0x9800 + 4]                   ; Store map at 0x9800+4
    xor esi, esi                            ; Zero entry count
.seek:
    cmp esi, 1024                           ; Do not proceed beyond
                                            ; 1024 entries
    jge .end                                ; - skip if true

    push edi                                ; Save EDI
    mov eax, 0xe820                         ; Set call number
    mov edx, 0x534D4150                     ; String: 'SMAP'
    mov ecx, 24                             ; Ask for 24-byte entries
    int 0x15                                ; Call BIOS
    jc .end                                 ; We're done already...
    cmp eax, 0x534D4150                     ; String : 'SMAP'
    jne .end                                ; We're also done already...

    pop edi                                 ; Restore EDI
    add edi, 24                             ; Move EDI to the next 24-byte slot
    inc esi                                 ; Increase entry count
    test ebx, ebx                           ; Test completion flag
    jnz .seek                               ; Continue if not complete
    ; --- fall-through --- ;

.end:
    mov [0x9800], esi                       ; Store entry count
    pop edi                                 ; Restore EDI

; Load second stage from the reserved sectors
read_s2:
    ; now what do I do?
    
    ; Expect >= 6 kB (>= 12 conventional sectors)
    mov ax, [ReservedSectors]               ; Get reserved sectors count
    mov dx, [BytesPerSector]                ; Get sector size
    mul dx                                  ; Get reserved area size (DX:AX)
    cmp dx, 0                               ; Check upper half (DX * 64 kB)
    jnz .read_s2_cont                       ; Skip process if the reserved area is obviously large (>= 64 kB)

    ; Check lower half (AX * 1 B)
    mov dx, 512                             ; Divide reserved area size by 512 B
    div dx                                  ; (divide value stored in AX by DX)
    cmp ax, 8                               ; Check quotient
    jl panic                                ; Panic if the reserved area is too small
    ; - fall-through - ;

.read_s2_cont:
    ; Read 12 conventional sectors
    lea bx, [stage_2]                       ; Point BIOS to buffer at 0x7e00
    mov ax, 0x020c                          ; Read 12 sectors
    mov cx, 0x0002                          ; Read from LBA 1 (C0, S2)
    mov dh, 0                               ; Read from head 0
    mov dl, [bootdev]                       ; Read from boot device
    int 0x13                                ; Call BIOS
    jc panic                                ; Panic on failure

; Enable A20 gate - fast
fast_a20:
    in al, 0x92                             ; Read from port 0x92
    test al, 2                              ; Check if A20 is already enabled
    jnz .fast_a20_after
    or al, 2                                ; Set A20 bit
    and al, 0xfe                            ; Clear fast reset bit
    out 0x92, al                            ; Write to port 0x92
.fast_a20_after:
    xchg bx, bx                              ; Breakpoint in Bochs 

; Enter 32-bit protected mode
enter_pm:
    ; Here we go...
    cli                                     ; Kill interrupts
    lgdt [gdtr]                             ; Load GDT address
    mov eax, cr0                            ; Read control register 0
    or eax, 1                               ; Set PE bit 
    mov cr0, eax                            ; Write back to CR0

    ; Perform far jump to segment 0x08 (described in the GDT)
    jmp 0x08:pm

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

align 16
[bits 32]
pm:
    mov ax, 0x10                            ; Point to kernel data segment

    ; - Set data segments
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    ; - reset stack
    mov ss, ax                              ; Set stack segment register
    mov esp, 0x7b00                         ; Reset stack pointer

    ; TODO: brief pre-jump checklist
    ; - maybe enable PAE (if present) and 
    ;   identity paging
    ; - establish C-like call stack, with
    ;   proper argument passing etc.
    ; - make it all fit without leaving
    ;   the MBR

    ; Pass arguments
    ; - zero-pad shorter arguments
    movzx eax, byte [bootdev]               ; Store boot device number
    push eax

    movzx eax, word [textmode]              ; Store active text mode
    push eax

    lea eax, [OemLabel]                     ; Store BPB location
    push eax

    ; Perform near call to loaded program
    call stage_2

    ; Halt (unreachable)
    cli
    hlt

; Variables
bootdev     db 0    ; Boot device (used predominantly by the BIOS)
textmode    dw 0    ; Text mode
errmsg      db "Boot failed. Replace boot device and press any key to restart.", 0

; Structures
; - Global descriptor table
; For now, focus on the kernel
gdt:
    ; Null descriptor (0x00)
    .null:
        dq 0                                ; 4 x 16 zeroes
    ; Kernel mode descriptor (0x08, 0x10)
    .kern_cs:
        dw 0xffff, 0, 0x9a, 0xcf            ; ...whatever this is
    .kern_ds:
        dw 0xffff, 0, 0x92, 0xcf            ; ...whatever this is
.end:

; - GDT pointer
gdtr:
    dw gdt.end - gdt - 1                    ; Size of GDT - 1
    dd gdt                                  ; Base of GDT
zidtr:
    ; - assume this area is zeroed out

times 510-($-$$) db 0                       ; Pad the boot record
dw 0xaa55                                   ; Boot signature

stage_2:                                    ; stage 2 entry point (0x7e00)