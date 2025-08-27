; magnetite_os - boot/src/vbr.asm
;
; The *absolute* initial stage of every boot sequence
; you can find out there in the wild.
;
; The task is simple (on paper): load the second-stage
; loader from a FAT16 volume, and run it.
; 
; Boot parameters are passed using
; a custom contract to save space.
;
; We'll be using BP to keep track of local variables.
;
; Refer to 'boot/src/defs.asm' for memory layout.
;
; TODO: if possible, add concrete error messages

; Include definitions
%include "boot/src/defs.asm"

[bits 16]
[org 0x7c00]

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
    cli                                     ; Disable interrupts
    xchg bx, bx                             ; Breakpoint

    ; Enforce flat addressing
    jmp 0:.start
.start:
    ; Zero out segment registers and initialize stack
    xor ax, ax
    mov ds, ax
    mov es, ax

    ; Initialize stack
    ; - hopefully, we have more than enough
    ;   space, and that the stack actually
    ;   moves downwards
    ; - set a 256 B buffer between bottom
    ;   of stack and boot sector start
    mov ss, ax
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
    xor dx, dx                              ; Zero DX
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
    xor dx, dx                              ; Zero DX
    mov ax, [ReservedSectors]               ; Load reserved sectors
    add ax, [HiddenSectors.low]             ; Add low word of hidden sectors count
    adc dx, [HiddenSectors.high]            ; Propagate carry

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
    ; FIXME: key bug point
    lea si, [bp + FIRST_RD_SECTOR_LOW]      ; Point to first RD sector (LE encoding)
    lea di, [bp + DAP_LBA_LOW]              ; Point to LBA in DAP (LE encoding)

    ; Copy 2 words
    times 2 movsw

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
    mov bx, read_buf                        ; Reset buffer pointer
    mov [bp + DAP_BUF_OFFSET], bx           ; Store it in DAP (DS = ES = 0)
    mov cx, 2                               ; Read just two sectors
    pusha                                   ; Save all GPRs
    call read_bootdev                       ; Read from boot drive
                                            ; (increments LBA by 2)
    ; --- AX and DX clobbered --- ;
    popa                                    ; Restore all GPRs

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
    mov di, bx
    lea si, [filename]
    mov cx, SIZEOF_83NAME                   ; Compare all 8+3 bytes
    rep cmpsb                               ; (here)
    je .done                                ; Break on success
    ; --- fall-through on failure --- ;

    add bx, SIZEOF_RDENTRY                  ; Increment address to buffer by entry size
    dec dx                                  ; Decrement entries counter

    pop cx                                  ; Restore CX
    loop .top                               ; Go back to top (decrement CX)
    ; --- fall-through on exhaustion --- ;

    jmp panic                               ; Give up on failure
.done:
    pop cx

parse_entry:
    xchg bx, bx
    ; TODO: maybe cheat using 32-bit registers

    ; CONTEXT 0
    ; - DI is inhereted from CONTEXT 1
    ; At this point, we should have the pointer
    ; to the relevant directory entry in DI

    ; Load cluster ID 0
    mov ax, [bx + RDE_FIRST_CLUSTER]        ; Load low word, as the high word
                                            ; is always zero in FAT12/FAT16
    mov di, ADDR_S2_LDR                     ; Point DI to target memory
.top:
    ; Step 1: check current cluster ID
    ; - expects cluster ID in AX
    mov cx, 0xffef                          ; Check if we're at the end of the chain
    sub cx, ax                              ; Subtract AX from CX
    jc .done                                ; If it underflows, we're done here

    push ax                                 ; Save AX

    ; Step 2
    ; Copy current cluster to target
    xor dx, dx                              ; Zero DX
    movzx cx, byte [SectorsPerCluster]      ; Store cluster size in CX
    sub ax, 2                               ; Subtract from cluster ID
    mul cx                                  ; Multiply by cluster size
    
    add ax, [bp + FIRST_DATA_SECTOR_LOW]    ; Add low word to AX
    adc dx, [bp + FIRST_DATA_SECTOR_HIGH]   ; Add high word to DX with carry
    jc .stop                                ; Give up on overflow

    mov [bp + DAP_BUF_OFFSET], di           ; Store target address in DAP
    mov [bp + DAP_LBA_LOW], ax              ; Store LBA low word
    mov [bp + DAP_LBA_MID_1], dx            ; Store LBA low middle word
    call read_bootdev                       ; Read from boot drive
    ; --- AX and DX clobbered --- ;
    xor dx, dx                              ; Zero DX
    mov ax, cx                              ; Store cluster size in AX
    mul word [BytesPerSector]               ; Multiply by sector size
    jc .stop                                ; Give up high word is set

    add di, ax                              ; Increment DI
    jnc .clear                              ; Continue on success
.stop:
    jmp panic
.clear:
    xchg bx, bx
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
    jnz .stop                               ; Give up if non-zero

    pop ax                                  ; Restore AX
    xor dx, dx                              ; Zero DX
    div si                                  ; Divide 0:AX by sector size
    ; (low quotient in AX, low remainder in DX)
    xchg bx, dx                             ; Exchange BX and DX
    add ax, [bp + FIRST_FAT_SECTOR_LOW]     ; Add low word of FAT LBA
    adc dx, [bp + FIRST_FAT_SECTOR_HIGH]    ; Add high word of FAT LBA
    jc panic                                ; Give up on overflow
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
    ; FIXME: potential redundancy
    mov [bp + LAST_ACCESSED_LOW], ax        ; Save low word of new LBA
    mov [bp + LAST_ACCESSED_HIGH], dx       ; Save high word of new LBA

    mov [bp + DAP_LBA_LOW], ax
    mov [bp + DAP_LBA_MID_1], dx

    mov di, read_buf                        ; Point DI to read buffer
    mov cx, 1                               ; Read just 1 sector

    mov [bp + DAP_BUF_OFFSET], di           ; Store value of DI in DAP
    call read_bootdev                       ; Read from bootdrive
    ; --- AX and DX clobbered --- ;
.read:
    mov ax, [read_buf + bx]                 ; Read cluster ID from calculated offset
    jmp .top                                ; Go back to top of loop
.done:
    mov al, [bootdev]
    push ADDR_S2_LDR                        ; Jump to second-stage loader
    ret                                     ; (here)

; --- Routines --- ;
; Read from boot drive
; Clobbers: AX, DX, SI
; - this one may be a little hard to read
; - BP = init. SP = 0x7b00, DS = ES = CS = 0
; Accepts:
; - CX: number of sectors to read
; - dap.lba: 32-bit LBA (incremented by the function)
; - dap.buf_offset: target buffer offset
; - dap.buf_segment: target buffer segment
read_bootdev:
    mov word [bp + DAP_NUM_SECTORS], 1      ; Load just 1 sector per iteration
.read:
    ; Read from disk
    mov dl, [bootdev]                       ; Load boot drive number into LD
    mov ah, 0x42                            ; Extended read
    lea si, [bp + DAP_FRAME]                ; Point SI to DAP (ES = 0)

    int 0x13                                ; Call BIOS
    test ah, ah                             ; Check return code
    jnz panic                               ; Break on error    
    ; --- fall-through --- ;

.cont:
    ; Move write pointer, accounting
    ; for segmentation
    ; - dirty trick
    mov ax, [BytesPerSector]                ; Load sector size
    add [bp + DAP_BUF_OFFSET], ax           ; Add it to buffer offset
    adc dx, 0                               ; Propagate carry
    shr dx, 15                              ; Make it MSB
    add [bp + DAP_BUF_SEGMENT], dx          ; Add it to buffer segment
    jc panic                                ; Give up on overflow

    ; Increment source LBA
    ; - cap usable LBA to 32 bits
    add word [bp + DAP_LBA_LOW], 1          ; Increment low word
    adc word [bp + DAP_LBA_MID_1], 0        ; Propagate carry to high word
    jc panic                                ; Give up on overflow

    loop .read                              ; Read one more sector (decrement CX)
    ; --- fall-through --- ;
.done:
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
    jz .stop
    int 0x10
    jmp .print
.stop:
    ; Halt without returning
    cli
    hlt

; Error messages
; NOTE: This consumes valuable space
errmsg              db "ERR", 0

; Target file name (8.3)
; - zero-terminated for good measure
filename            db "BOOT1   BIN", 0

; Variables
bootdev:            db 0                    ; Boot drive number

times 510-($-$$) db 0                       ; Pad the boot record
dw 0xaa55                                   ; Boot signature

read_buf:                                   ; Read buffer at 0x7e00 
                                            ; (4 kB, with overrun)