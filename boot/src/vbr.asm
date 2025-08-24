; magnetite_os - boot/vbr.asm
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
; * 0x07e00-0x08bff - read buffer (4 kB, accept overrun in initial stage)
; * 0x09000-0x0a7ff - second-stage loader
; * 0x0a800-0x10fff - E820 memory map
; * 0x11000-0x1ffff - minimum usable memory (128 kB)
; * 0x20000-0x7ffff - maximum usable memory (>128 kB)
;
; TODO: if possible, add concrete error messages

[bits 16]
[org 0x7c00]

ADDR_S2_LDR         equ 0x9000
ADDR_E820_MAP       equ 0xa800

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

; Walk root directory
walk_root_dir:
    xchg bx, bx                             ; Bochs breakpoint
    xor dx, dx                              ; Zero DX (buffer size)

    ; Load parameters to DAP
    mov ax, [first_root_dir_sector.low]
    mov bx, [first_root_dir_sector.high]
    mov [dap.lba.low], ax
    mov [dap.lba.mid1], bx

    ; CONTEXT 0
    ; - expect CX, DX, SI and DI to be clobbered
    mov cx, [RootDirEntries]                ; Set CX with entry count
.top:
    ; Check if buffer is exhausted
    test dx, dx                             ; Check if DX > 0
    jnz .skip_read                          ; Skip reading if true
    ; --- fall-through --- ;

    ; Read more entries on exhaustion
    ; - also performed on the first iteration
    mov di, read_buf                        ; Reset buffer pointer
    mov [dap.buf_offset], di                ; Store it in DAP
    mov cx, 2                               ; Read just two sectors
    call read_bootdev                       ; Read from boot drive
                                            ; (increments LBA by 2)

    ; Calculate entries per sector to
    ; mark the buffer as replenished
    mov dx, [BytesPerSector]                ; Store sector size in DX
    shr dx, 4                               ; Divide it by 16

.skip_read:
    ; CONTEXT 1
    ; Perform a quick-and-dirty check
    ; of the entry's 8.3 filename
    ; - target name pointer in SI,
    ;   buffer pointer in DI
    pusha                                   ; Save all GPRs
    mov cx, SIZEOF_83NAME                   ; Compare all 8+3 bytes
    rep cmpsb                               ; (here)
    popa                                    ; Restore all GPRs
    je parse_entry                          ; Break on success
    ; --- fall-through on failure --- ;

    add di, SIZEOF_RDENTRY                  ; Increment address to buffer by entry size
    dec dx                                  ; Decrement entries counter

    loop .top                               ; Go back to top (decrement CX)
    ; --- fall-through on exhaustion --- ;

    jmp panic                               ; Give up on failure

parse_entry:
    ; CONTEXT 0
    ; - DI is inhereted from CONTEXT 1

    ; At this point, we should have the pointer
    ; to the relevant directory entry in DI

    ; TODO

; --- Routines --- ;
; Read from boot drive
; - this one may be a little hard to read
; Accepts:
; - CX: number of sectors to read
; - dap.lba: source LBA (incremented by the function)
; - dap.buf_offset: target buffer offset
; - dap.buf_segment: target buffer segment
read_bootdev:
    xchg bx, bx                             ; Bochs breakpoint
    xor dx, dx                              ; Zero DX

    pusha                                   ; Preserve GPRs

    ; Set ES = CS
    mov ax, cs
    mov es, ax

    mov ax, 1                               ; Load just 1 sector per iteration
    mov [dap.num_sectors], ax               ; (store it in DAP)
.read:
    ; Clobbers: AX, BX, DX, SI
    ; Read from disk
    xor ax, ax                              ; Zero AX
    xor bx, bx                              ; Zero BX (for good measure)
    mov dl, [bootdev]                       ; Load boot drive number into LD
    mov ah, 0x42                            ; Extended read
    mov si, dap                             ; Point SI to DAP (ES = 0)

    clc                                     ; Clear CF
    int 0x13                                ; Call BIOS
    jc .stop                                ; Stop on failure

    test ah, ah                             ; Check return code
    jz .cont                                ; Continue on success
    ; --- fall-through --- ;
.stop:
    popa
    jmp panic
.cont:
    ; Move write pointer
    mov ax, [BytesPerSector]
    add [dap.buf_offset], ax

    ; Increment source LBA
    add [dap.lba.low], ax
    adc word [dap.lba.mid1], 0
    adc word [dap.lba.mid2], 0
    adc word [dap.lba.high], 0

    loop .read
    ; --- fall-through --- ;
    popa
    ret

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
                                            ; (4 kB, with overrun)