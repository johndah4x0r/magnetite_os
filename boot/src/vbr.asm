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
; We'll be using BP to keep track of local variables.
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

ADDR_S2_LDR                 equ 0x9000
ADDR_E820_MAP               equ 0xa800

SIZEOF_RDENTRY              equ 32
SIZEOF_83NAME               equ 11

FRAME                       equ -18
ROOT_DIR_SECTORS            equ FRAME + 0
FIRST_FAT_SECTOR_LOW        equ FRAME + 2
FIRST_FAT_SECTOR_HIGH       equ FRAME + 4
FIRST_RD_SECTOR_LOW         equ FRAME + 6
FIRST_RD_SECTOR_HIGH        equ FRAME + 8
FIRST_DATA_SECTOR_LOW       equ FRAME + 10
FIRST_DATA_SECTOR_HIGH      equ FRAME + 12
LAST_ACCESSED_LOW           equ FRAME + 14
LAST_ACCESSED_HIGH          equ FRAME + 16

DAP_FRAME                   equ FRAME - 16
DAP_SIZE                    equ DAP_FRAME + 0
DAP_NUM_SECTORS             equ DAP_FRAME + 2
DAP_BUF_OFFSET              equ DAP_FRAME + 4
DAP_BUF_SEGMENT             equ DAP_FRAME + 6
DAP_LBA_LOW                 equ DAP_FRAME + 8
DAP_LBA_MID_1               equ DAP_FRAME + 10
DAP_LBA_MID_2               equ DAP_FRAME + 12
DAP_LBA_HIGH                equ DAP_FRAME + 14

LOWEST_FRAME                equ DAP_FRAME
GUARD_SIZE                  equ 2

ALLOC_SIZE                  equ -LOWEST_FRAME + GUARD_SIZE

RDE_FIRST_CLUSTER           equ 26

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
    xor dx, dx
    mov ds, dx
    mov es, dx

    ; Initialize stack
    ; - hopefully, we have more than enough
    ;   space, and that the stack actually
    ;   moves downwards
    ; - set a 256 B buffer between bottom
    ;   of stack and boot sector start
    mov ss, dx
    mov sp, 0x7b00

    ; Initialize frames
    ; (don't you hate this?)
    cld                                     ; Clear DF
    mov bp, sp                              ; Set BP = SP (= 0x7c00, maybe?)
    sub sp, ALLOC_SIZE                      ; Allocate this many bytes (+2 bytes guard)

    lea di, [bp + DAP_FRAME]                ; Point DI to DAP
    mov cx, 8                               ; Zero out 8 words
    rep stosw                               ; (here)

    mov word [bp + DAP_SIZE], 0x0010        ; Set DAP size

    ; Set flags
    sti                                     ; Enable interrupts

    ; Store device number
    mov [bootdev], dl

; Check whether drive extensions
; are present
check_ext:
    mov ah, 0x41                            ; Extensions check
    mov bx, 0x55aa                          ; Input bit pattern

    xor cx, cx                              ; Zero CX
    int 0x13                                ; Call BIOS
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
    ; Calculate number of root
    ; directory sectors
    ; FIXME: This may crash in Bochs
    mov ax, [RootDirEntries]                ; Store number of root directory entries
    mov bx, 32                              ; Explicitly multiply by 32
    mul bx                                  ; (here)

    mov bx, [BytesPerSector]                ; Store sector size
    lea cx, [bx - 1]                        ; Decrement and store in CX
    add ax, cx                              ; Add into AX to over-count
    adc dx, 0                               ; Propagate carry

    div bx                                  ; Divide AX by sector size
    mov [bp + ROOT_DIR_SECTORS], ax         ; Store quotient and ignore remainder

    ; Calculate first FAT sector
    mov ax, [HiddenSectors.low]             ; Load low word of hidden sectors count
    mov dx, [HiddenSectors.high]            ; Load high word of hidden sectors count

    add ax, [ReservedSectors]               ; Add reserved sectors count
    adc dx, 0                               ; Propagate carry

    mov [bp + FIRST_FAT_SECTOR_LOW], ax     ; Store low word of sum
    mov [bp + FIRST_FAT_SECTOR_HIGH], dx    ; Store high word of sum

    ; Calculate first root directory sector
    ; - this is done by calculating where
    ; the FAT region ends (there are usually
    ; more than one FATs in most volumes)
    mov ax, [SectorsPerTab]                 ; Store FAT size
    mul word [FatCount]                     ; Multiply it by the FAT count

    add ax, [bp + FIRST_FAT_SECTOR_LOW]     ; Add low word of FAT LBA
    adc dx, [bp + FIRST_FAT_SECTOR_HIGH]    ; Add high word of FAT LBA with carry

    mov [bp + FIRST_RD_SECTOR_LOW], ax      ; Store low word of result
    mov [bp + FIRST_RD_SECTOR_HIGH], dx     ; Store high word of result

    ; Calculate first data sector
    add ax, [bp + ROOT_DIR_SECTORS]         ; Add number of root directory sectors
    adc dx, 0                               ; Propagate carry
    
    mov [bp + FIRST_DATA_SECTOR_LOW], ax    ; Store low word of sum
    mov [bp + FIRST_DATA_SECTOR_HIGH], dx   ; Store high word of sum

; Walk root directory
walk_root_dir:
    xor dx, dx                              ; Zero DX (buffer size)

    ; Load parameters to DAP
    lea si, [bp + FIRST_RD_SECTOR_LOW]      ; Point to first RD sector (LE encoding)
    lea di, [bp + DAP_LBA_LOW]              ; Point to LBA in DAP (LE encoding)
    movsd                                   ; Copy 32-bit value

    ; CONTEXT 0
    ; - expect CX, DX, SI and DI to be clobbered
    mov cx, [RootDirEntries]                ; Set CX with entry count
.top:
    push cx                                 ; Save CX

    ; Check if buffer is exhausted
    test dx, dx                             ; Check if DX > 0
    jnz .skip_read                          ; Skip reading if true
    ; --- fall-through --- ;

    ; Read more entries on exhaustion
    ; - also performed on the first iteration
    mov di, read_buf                        ; Reset buffer pointer
    mov [bp + DAP_BUF_OFFSET], di           ; Store it in DAP (DS = ES = 0)
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
    mov cx, SIZEOF_83NAME                   ; Compare all 8+3 bytes
    rep cmpsb                               ; (here)
    je .done                                ; Break on success
    ; --- fall-through on failure --- ;

    add di, SIZEOF_RDENTRY                  ; Increment address to buffer by entry size
    dec dx                                  ; Decrement entries counter

    pop cx                                  ; Restore CX
    loop .top                               ; Go back to top (decrement CX)
    ; --- fall-through on exhaustion --- ;

    jmp panic                               ; Give up on failure
.done:
    pop cx

parse_entry:
    ; TODO: maybe cheat using 32-bit registers

    ; CONTEXT 0
    ; - DI is inhereted from CONTEXT 1
    ; At this point, we should have the pointer
    ; to the relevant directory entry in DI

    ; Load cluster ID for the first cluster
    mov ax, [di + RDE_FIRST_CLUSTER]        ; Load low word, as the high word
                                            ; is always zero in FAT12/FAT16
    mov di, ADDR_S2_LDR                     ; Point DI to target memory
.top:
    ; Step 1: check current cluster ID
    ; - expects cluster ID in AX
    cmp ax, 0xfff0                          ; Check if we're at the end of the chain
    jge .done                               ; If it is, we're done here

    push ax                                 ; Save AX

    ; Step 2
    ; Load current cluster to target
    xor dx, dx                              ; Zero DX
    mov cx, [SectorsPerCluster]             ; Store cluster size in CX
    sub ax, 2                               ; Subtract from cluster ID
    mul cx                                  ; Multiply by cluster size
    
    add ax, [bp + FIRST_DATA_SECTOR_LOW]    ; Add low word to AX
    adc dx, [bp + FIRST_DATA_SECTOR_HIGH]   ; Add high word to DX with carry
    jc panic                                ; Give up on overflow

    mov [bp + DAP_BUF_OFFSET], di           ; Store target address in DAP
    call read_bootdev                       ; Read from boot drive

    xor dx, dx                              ; Zero DX
    mov ax, cx                              ; Store cluster size in AX
    mul word [BytesPerSector]               ; Multiply by sector size
    test dx, dx                             ; Check high word
    jnz panic                               ; Give up if set

    add di, ax                              ; Increment DI
    jc panic                                ; Give up on overflow

    pop ax                                  ; Restore AX

    ; Step 3
    ; Calculate offset for next cluster ID
    ; in FAT (stored in DX:AX)
    xor dx, dx                              ; Zero DX
    mov cx, 2                               ; Multiply AX by 2
    mul cx                                  ; (here)

    ; Calculate relative LBA and offset
    mov si, [BytesPerSector]                ; Store sector size in SI
    push ax                                 ; Save AX
    xor ax, ax                              ; Zero AX
    div si                                  ; Divide DX:0 by sector size
    mov bx, ax                              ; Save high quotient in BX

    test dx, dx                             ; Check high remainder
    jnz panic                               ; Give up if non-zero

    pop ax                                  ; Restore AX
    xor dx, dx                              ; Zero DX
    div si                                  ; Divide 0:AX by sector size
    ; (low quotient in AX, low remainder in DX)
    xchg bx, dx                             ; Exchange BX and DX
    ; (LBA in DX:AX, offset in 0:BX)
    ; (now, who in the right mind would use
    ; 64 kiB sectors?)

    ; Step 4
    ; Read next cluster ID from FAT
    ; (LBA in DX:AX, offset in BX)
    cmp ax, [bp + LAST_ACCESSED_LOW]        ; Compare low word of cached LBA
    jne .replenish                          ; Replenish buffer on mismatch
    cmp dx, [bp + LAST_ACCESSED_HIGH]       ; Compare high word of cached LBA
    je .read                                ; Proceed on match
    ; --- fall-through on mismatch --- ;
.replenish:
    mov [bp + LAST_ACCESSED_LOW], ax        ; Save low word of new LBA
    mov [bp + LAST_ACCESSED_HIGH], dx       ; Save high word of new LBA

    mov di, read_buf                        ; Point DI to read buffer
    mov cx, 1                               ; Read just 1 sector

    mov [bp + DAP_BUF_OFFSET], di           ; Store value of DI in DAP
    call read_bootdev                       ; Read from bootdrive
.read:
    mov ax, [read_buf + bx]                 ; Read cluster ID from calculated offset
    jmp .top                                ; Go back to top of loop
.done:
    jmp ADDR_S2_LDR                         ; Jump to second-stage loader

; --- Routines --- ;
; Read from boot drive
; - this one may be a little hard to read
; - BP = init. SP = 0x7b00, DS = ES = CS = 0
; Accepts:
; - DX:AX: 32-bit LBA
; - CX: number of sectors to read
; - dap.lba: source LBA (incremented by the function)
; - dap.buf_offset: target buffer offset
; - dap.buf_segment: target buffer segment
read_bootdev:
    pusha                                   ; Preserve GPRs

    mov word [bp + DAP_NUM_SECTORS], 1      ; Load just 1 sector per iteration
    mov [bp + DAP_LBA_LOW], ax              ; Load low word of LBA mov [bp +
    mov [bp + DAP_LBA_MID_1], dx            ; Load high word of LBA

    ; Zero out upper 32 bits of DAP LBA
    xor ax, ax
    mov [bp + DAP_LBA_MID_2], ax
    mov [bp + DAP_LBA_HIGH], ax
.read:
    ; Clobbers: AX, DX, SI
    ; Read from disk
    mov dl, [bootdev]                       ; Load boot drive number into LD
    mov ah, 0x42                            ; Extended read
    lea si, [bp + DAP_FRAME]                ; Point SI to DAP (ES = 0)

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
    add [bp + DAP_BUF_OFFSET], ax

    ; Increment source LBA
    ; - cap usable LBA to 32 bits
    add word [bp + DAP_LBA_LOW], 1          ; Increment low word
    adc word [bp + DAP_LBA_MID_1], 0        ; Propagate carry to high word
    jc panic                                ; Give up on overflow

    loop .read                              ; Read one more sector (decrement CX)
    ; --- fall-through --- ;
.done:
    popa
    ret

; Print error message to screen, then reset
panic:
    xchg bx, bx                             ; Breakpoint in Bochs
    sti                                     ; Enable interrupts 

    ; Write error string to screen
    mov si, errmsg                          ; Pointer to boilerplate message
    mov ah, 0x0e
.print:
    lodsb
    test al, al
    jz .spin
    int 0x10
    jmp .print
.spin:
    rep nop                                 ; Pause
    jmp .spin


; Error messages
; NOTE: This consumes valuable space
errmsg              db "Change boot drive and reset", 0

; Target file name (8.3)
; - zero-terminated for good measure
filename            db "BOOT    BIN", 0

; Variables
bootdev:            db 0                    ; Boot drive number

times 510-($-$$) db 0                       ; Pad the boot record
dw 0xaa55                                   ; Boot signature

read_buf:                                   ; Read buffer at 0x7e00 
                                            ; (4 kB, with overrun)