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
;   the contract between the MBR and the second-stage
;   loader.
;
; We'll avoid using i386+ extensions in the MBR
; whenever possible - no cheating!
;
; Low Memory is used as follows:
; * 0x00500-0x07af0 - stack (to be relocated)
; * 0x07b00-0x07bff - guard region (not enforced)
; * 0x07c00-0x07dff - MBR
; * 0x07e00-0x07fff - read buffer
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
    dw 13
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
    ; - hopefully we have more than enough space
    ; and that the stack actually moves downwards.
    ; - set a 256 B buffer between bottom of stack and
    ; boot sector start
    mov ss, bx
    mov sp, 0x7b00
    ; Restore interrupts
    sti

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
    jnc .cont
    ; - fall-through - ;

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

; Check whether drive extensions are present
check_ext:
    mov ah, 0x41                            ; Extensions check
    mov bx, 0x55aa                          ; Input bit pattern
    mov dl, [bootdev]                       ; Read back boot device

    clc                                     ; Clear CF
    int 0x13                                ; Call BIOS
    jc .stop                                ; Stop if CF is set

    cmp bx, 0xaa55                          ; Assert that the bit pattern is altered
    je .end                                 ; Continue if altered
    ; --- fall-through --- ;
.stop:
    ; TODO
    jmp panic
.end:

; Calculate where the root directory and
; the data are located
compute_sectors:
    xchg bx, bx                             ; Breakpoint

    ; Calculate number of root directory sectors
    ; FIXME: This may crash in Bochs
    xor ax, ax
    xor bx, bx

    mov ax, [RootDirEntries]                ; Store number of root directory entries
    shl ax, 5                               ; Multiply it by 32
    mov bx, [BytesPerSector]                ; Store sector size
    div bx                                  ; Divide AX by sector size

    mov [root_dir_sectors], ax              ; Store result
    xor dx, dx                              ; (ignore remainder)

    ; Calculate first root directory sector
    mov ax, [SectorsPerTab]                 ; Store FAT size
    mov bx, [FatCount]                      ; Multiply it by the FAT count
    mul bx                                  ; (here)

    mov [first_root_dir_sector], ax         ; Store result
    xor dx, dx                              ; (ignore remainder)

    ; Calculate first data sector
    add ax, [root_dir_sectors]              ; Add number of root directory sectors
    mov [first_data_sector], ax             ; Store result

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

    ; If all else fails, force a triple fault
    ; - use 0x7b00 as source
    lidt [0x7b00]
    int 0x00

; Error messages
; NOTE: This consumes valuable space
errmsg              db "Boot failed. Replace boot device and reset.", 0

; Variables
bootdev:
    db 0
root_dir_sectors:
    dw 0
first_root_dir_sector:
    dw 0
first_data_sector:
    dw 0

; - drive access packet
d_packet:           db 0x10, 0              ; Size field is 0x10, unused field is 0x00
    .num_sectors    dw 0                    ; Number of sectors to be accessed
    .buf_offset     dw 0                    ; Offset to buffer
    .buf_segment    dw 0                    ; Segment to buffer
    .lba_low        dw 0                    ; Lower half of LBA
    .lba_high       dw 0                    ; Higher half of LBA

times 510-($-$$) db 0                       ; Pad the boot record
dw 0xaa55                                   ; Boot signature

read_buf:                                   ; Read buffer at 0x7e00