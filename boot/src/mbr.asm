; magnetite_os - boot/boot0.asm
;
; The *absolute* initial stage of every boot sequence
; you can find out there in the wild.
;
; Initialize the system, load second stage into 0x8000,
; enable protected mode and transfer control to second stage.
; 
; Boot parameters are passed using a cdecl-like ABI.
; - This isn't strictly necessary, as we *do* control
;   the contract between the boot record and the 
;   second-stage loader.
;
; If a CPU is older than the i386, then clearly we've
; gone too far back in time.
;
; Low Memory is used as follows:
; * 0x00500-0x07af0 - stack (to be relocated)
; * 0x07b00-0x07bff - guard region (not enforced)
; * 0x07c00-0x07dff - boot record
; * 0x07e00-0x07fff - read buffer (accept overrun in initial stage)
; * 0x08000-0x097ff - second-stage loader
; * 0x09800-0x0ffff - E820 memory map
;   (+0x00: 64-bit base)
;   (+0x08: 64-bit size)
;   (+0x10: E820 array)
; * 0x10000-0x1ffff - minimum usable memory (=128 kB)
; * 0x20000-0x7ffff - maximum usable memory (>128 kB)
;
; TODO: if possible, add concrete error messages

[bits 16]
[org 0x7c00]

ADDR_S2_LDR         equ 0x8000
ADDR_E820_MAP       equ 0x9800

SIZEOF_RDENTRY      equ 32
SIZEOF_83NAME       equ 11

; We'll just assume that the very first bytes
; of the target boot record are 'EB 3C 90',
; and work from there.

jmp short _start
nop

; Dummy BIOS parameter block (DOS 4.0)
; - Used for references, skipped when overwriting 
;   target MBR (or should I say, target VBR)
;
; ...doesn't stop me from decorating it, though...
OemLabel:           db "MGNTTEOS"
BytesPerSector:     dw 512
SectorsPerCluster:  db 4
ReservedSectors:    dw 13
FatCount:           db 2
RootDirEntries:     dw 512
SectorsCount:       dw 0
MediumType:         db 0xf8
SectorsPerTab:      dw 64
SectorsPerTrack:    dw 16
Heads:              dw 8
HiddenSectors:
    .low            dw 0
    .high           dw 0
LargeSectors:
    .low            dw 0
    .high           dw 1
DriveNumber:        dw 0
Signature:          db 0x29
VolumeId:           dd 0x1337c0de
VolumeLabel:        db "MAGNETITEOS"
FileSystem:         db "FAT16   "

; --- Main routine --- ;
_start:
    ; now what do I do?

    ; The first 2 instructions serve as landmarks
    ; to determine where the BPB ends and where bootstrap
    ; code starts.
    cli                                     ; FA - Kill interrupts
    xchg bx, bx                             ; 87 DB - Bochs breakpoint

    ; Enforce flat addressing
    jmp 0:.start
.start:
    ; Zero out segment registers and initialize stack
    xor bx, bx
    mov ds, bx
    mov es, bx

    ; Initialize stack
    ; - hopefully, we have more than enough
    ;   space, and that the stack actually
    ;   moves downwards
    ; - set a 256 B buffer between bottom
    ;   of stack and boot sector start
    mov ss, bx
    mov sp, 0x7b00

    ; Set flags
    sti                                     ; Enable interrupts
    cld                                     ; Clear direction flag

    ; Store device number
    mov [bootdev], dl

; Find storage device geometry
; - not quite useful when LBA is being
;   enforced, but *what gives?*
; - if not strictly necessary, then
;   this is just dead weight
calc_geometry:
    clc                                     ; Clear CF
    mov ah, 0x08                            ; Read drive parameters
    int 0x13                                ; Call BIOS
    jc .stop                                ; Stop if CF is set

    test ah, ah                             ; Check return code
    jz .cont                                ; Proceed on success 
    ; - fall-through - ;

.stop:
    ; TODO
    jmp panic
.cont:
    xchg bx, bx

    and cx, 0x3f
    mov [SectorsPerTrack], cx
    movzx dx, dh
    inc dx
    mov [Heads], dx
    ; - fall-through - ;

; Check whether drive extensions
; are present
check_ext:
    mov ah, 0x41                            ; Extensions check
    mov bx, 0x55aa                          ; Input bit pattern
    mov dl, [bootdev]                       ; Read back boot device

    clc                                     ; Clear CF
    int 0x13                                ; Call BIOS
    jc .stop                                ; Stop if CF is set

    test cx, 1                              ; Check whether we can use the DAP
    jz .stop                                ; Stop if not

    cmp bx, 0xaa55                          ; Assert that the bit pattern is altered
    je .end                                 ; Continue if altered
    ; --- fall-through --- ;
.stop:
    ; TODO
    jmp panic
.end:

; Calculate where the root directory and
; the data are located
; TODO: For efficiency reasons, read only
; a few clusters at a time.
compute_sectors:
    xchg bx, bx                             ; Breakpoint

    ; Calculate number of root
    ; directory sectors
    ; FIXME: This may crash in Bochs
    xor dx, dx                              ; Clear DX
    mov ax, [RootDirEntries]                ; Store number of root directory entries
    mov bx, 32                              ; Explicitly multiply by 32
    mul bx                                  ; (here)

    mov bx, [BytesPerSector]                ; Store sector size
    mov cx, bx                              ; Copy into CX
    dec cx                                  ; Decrement CX by one

    add ax, cx                              ; Add into AX to over-count
    adc dx, 0                               ; Propagate carry

    div bx                                  ; Divide AX by sector size

    mov [root_dir_sectors], ax              ; Store quotient and ignore remainder

    ; Calculate first FAT sector
    mov ax, [HiddenSectors.low]             ; Load low word of hidden sectors count
    mov dx, [HiddenSectors.high]            ; Load high word of hidden sectors count

    add ax, [ReservedSectors]               ; Add reserved sectors count
    adc dx, 0                               ; Propagate carry

    mov [first_fat_sector.low], ax          ; Store low word of sum
    mov [first_fat_sector.high], dx         ; Store high word of sum

    ; Calculate first root directory sector
    ; - this is done by calculating where
    ; the FAT region ends (there are usually
    ; more than one FATs in most volumes)
    mov ax, [SectorsPerTab]                 ; Store FAT size
    mov bx, [FatCount]                      ; Multiply it by the FAT count
    mul bx                                  ; (here)

    add ax, [first_fat_sector.low]          ; Add low word of FAT LBA
    adc dx, [first_fat_sector.high]         ; Add high word of FAT LBA with carry
    ; FIXME: who deals with overflows at DX?

    mov [first_root_dir_sector.low], ax     ; Store low word of result
    mov [first_root_dir_sector.high], dx    ; Store high word of result

    ; Calculate first data sector
    add ax, [root_dir_sectors]              ; Add number of root directory sectors
    adc dx, 0                               ; Propagate carry

    mov [first_data_sector.low], ax         ; Store low word of sum
    mov [first_data_sector.high], dx        ; Store high word of sum

; Walk the root directory and locate
; the target file
; - this one may be a little hard to read
walk_root_dir:
    xor dx, dx                              ; Zero DX

    ; Load parameters from variables
    ;
    ; TODO:
    ; Account for high count bits.
    ; Use i386+ extensions if
    ; functionality requires it
    mov bx, read_buf                        ; Load address to read buffer
    mov cx, [root_dir_sectors]              ; Load number of root directory sectors
    mov dl, [bootdev]                       ; Load boot device number

    ; Store parameters into DAP
    mov [dap.buf_offset], bx                ; Store buffer offset
    mov [dap.num_sectors], cx               ; Store number of sectors
    mov [dap.buf_segment], ds               ; Store buffer segment (DS = ES = 0)

    mov ax, [first_root_dir_sector.low]     ; Load low word into AX
    mov bx, [first_root_dir_sector.high]    ; Load high word into BX 
    mov [dap.lba.low], ax
    mov [dap.lba.mid1], bx

    ; Read from disk
    xor ax, ax                              ; Zero AX
    xor bx, bx                              ; Zero BX (for good measure)
    mov ah, 0x42                            ; Extended read
    mov si, dap                             ; Point SI to DAP

    clc                                     ; Clear CF
    int 0x13                                ; Call BIOS
    jc .stop                                ; Stop on failure

    test ah, ah                             ; Check return code
    jz .cont                                ; Continue on success
    ; --- fall-through --- ;
.stop:
    ; TODO
    jmp panic
.cont:
    ; CONTEXT 0
    ; - expect BX and CX to be clobbered
    mov bx, read_buf                        ; Load address to top of read buffer
    mov cx, [RootDirEntries]                ; Load number of root directory entries
.top:
    ; CONTEXT 1
    ; - break-on-success loop
    ; variants:
    ; - BX (entry pointer)
    ; - CX (remaining entries counter)
    ; - DL (first byte)
    ; - DH (name match flag; context 2)
    ; invariants:
    ; - SI (target filename)
    ; undefined:
    ; - AX ()
    mov dl, byte [bx]                       ; Check first byte
                                            ; (proof that BX is favored for mem-ops)

    cmp dl, 0x00                            ; (free/last entry)
    je .next

    cmp dl, 0x2e                            ; (dot entry)
    je .next
    
    cmp dl, 0xe5                            ; (deleted entry)
    je .next

    ; --- fall-through --- ;
    ; Compare file name to target file name
    ; - expect AX, BX, CX, DX and SI to be 
    ;   clobbered
    push bx                                 ; Save BX (top of entry)
    push cx                                 ; Save CX (remaining entries)

    ; CONTEXT 2
    mov cx, SIZEOF_83NAME                   ; Load size of name (decreasing counter)
    mov si, filename                        ; Load address to target file name
    xor dh, dh                              ; Zero DH (name match flag; unset)
.name_cmp:
    ; CONTEXT 2
    ; - break-on-failure loop
    ; variants:
    ; - BX (character pointer), 
    ; - CX (character index),
    ; - DL (character byte),
    ; - DH (name match flag)
    ; - SI (target pointer)
    ; invariants: 
    ; - BX (top of entry; context 1)
    ; clobbers:
    ; - AL (target character byte)
    mov dl, byte [bx]                       ; Read character from current entry
                                            ; (again, proof that BX is favored for mem-ops)

    lodsb                                   ; Load byte into AL from SI and increment it
    cmp dl, al                              ; Compare read character to target character
    jne .name_cmp_end                       ; Clean up if characters do not match
    ; --- fall-through on match --- ;

    inc bx                                  ; Increase BX (point to next character)
    loop .name_cmp                          ; Continue comparison while CX > 0
    ; --- fall-through on exhaustion --- ;

    ; Past this point, the names are
    ; guaranteed to be equal
    mov dh, 1                               ; Set name match flag
.name_cmp_end:
    ; END CONTEXT 2
    pop cx                                  ; Restore CX (remaining entries)
    pop bx                                  ; Restore BX (top of entry)

    ; CONTEXT 1
    test dh, dh                             ; Check if the match flag is set
    jnz .end                                ; End search if a match is found
    ; --- fall-through on fail --- ;

.next:
    ; IS CONTEXT 1
    add bx, SIZEOF_RDENTRY                  ; Increment address to buffer by entry size
    loop .top                               ; Return to top of loop (decrementing CX)
    ; --- fall-through on exhaustion --- ;

.end:
    ; END CONTEXT 1
    ; CONTEXT 0
    ; - BX is inhereted from CONTEXT 1
    test dh, dh                             ; Check if the match flag is set
    jz panic                                ; Give up if unset (file not found)
    ; --- fall-through if DH is set --- ;

parse_entry:
    ; At this point, we should have the pointer
    ; to the relevant directory entry in BX

    ; TODO

; --- Routines --- ;
; Print string to screen
; Accepts: SI (pointer to string)
; Assumes: IF = 1
print:
    push ax                                 ; Save AX
    mov ah, 0x0e                            ; BIOS teletype function
.cont:
    lodsb                                   ; Read 1 byte from SI, then shift
    test al, al                             ; End of string (zero-terminated)
    jz .done
    int 0x10                                ; Call BIOS
    jmp .cont                               ; Resume loop
.done:
    pop ax                                  ; Restore AX
    ret                                     ; Return to caller

; Print error message to screen, then reset
panic:
    xchg bx, bx                             ; Breakpoint in Bochs
    sti                                     ; Enable interrupts 

    ; Write error string to screen
    mov si, errmsg                          ; Pointer to boilerplate message
    call print                              ; Call print function

    xor ax, ax                              ; Clear AX
    int 0x16                                ; Wait for keystroke
    int 0x19                                ; Reboot system

    ; If all else fails, force a 
    ; triple fault
    ; - use 0x7b00 as source
    lidt [0x7b00]
    int 0x00

; Error messages
; NOTE: This consumes valuable space
errmsg              db "Replace boot device and reset.", 0

; Target file name (8.3)
; - zero-terminated for good measure
filename            db "BOOT    BIN", 0

; Variables
bootdev:            db 0                    ; Boot drive number
root_dir_sectors:   dw 0                    ; Number of root directory sectors
first_fat_sector:
    .low            dw 0                    ; Word 0 of FAT LBA
    .high           dw 0                    ; Word 1 of FAT LBA
first_root_dir_sector:
    .low            dw 0                    ; Word 0 of root directory LBA
    .high           dw 0                    ; Word 1 of root directory LBA
first_data_sector:
    .low            dw 0                    ; Word 0 of data region LBA
    .high           dw 0                    ; Word 1 of data region LBA

; Drive access packet (should be 16 bytes)
dap:                db 0x10, 0              ; (+2) Size field is 0x10, unused field is 0x00
    .num_sectors    dw 0                    ; (+2) Number of sectors to be accessed
    .buf_offset     dw 0                    ; (+2) Offset to buffer
    .buf_segment    dw 0                    ; (+2) Segment to buffer

; - DAP LBA (64-bit value)
;   Low and middle words are 16-bit
;   for DX:AX addressing (if needed)
.lba:
    .lba.low        dw 0                    ; (+2) Word 0 of LBA
    .lba.mid1       dw 0                    ; (+2) Word 1 of LBA
    .lba.mid2       dw 0                    ; (+2) Word 2 of LBA
    .lba.high       dw 0                    ; (+2) Word 3 of LBA


times 510-($-$$) db 0                       ; Pad the boot record
dw 0xaa55                                   ; Boot signature

read_buf:                                   ; Read buffer at 0x7e00