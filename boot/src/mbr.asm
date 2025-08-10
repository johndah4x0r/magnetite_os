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

    ; Enforce flat addressing
    jmp 0:.start
.start:
    ; Restore interrupts
    sti

    ; - Store device number here - ;
    mov [bootdev], dl
    cmp dl, 0
    je .next

    ; Find storage device geometry
    mov ah, 8
    int 0x13
    jc panic
    xchg bx, bx
    and cx, 0x3f
    mov [SectorsPerTrack], cx
    movzx dx, dh
    inc dx
    mov [Heads], dx
    ; - fall-through - ;
.next:

; TODO: make self-contained MBR

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

    mov [RootDirSectors], ax                ; Store result
    xor dx, dx                              ; (ignore remainder)

    ; Calculate first root directory sector
    mov ax, [SectorsPerTab]                 ; Store FAT size
    mov bx, [FatCount]                      ; Multiply it by the FAT count
    mul bx                                  ; (here)

    mov [FirstRootDirSector], ax            ; Store result
    xor dx, dx                              ; (ignore remainder)

    ; Calculate first data sector
    add ax, [RootDirSectors]                ; Add number of root directory sectors
    mov [FirstDataSector], ax               ; Store result

; --- Routines --- ;
panic:
    xchg bx, bx                             ; Breakpoint in Bochs

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
RootDirSectors:
    dw 0
FirstRootDirSector:
    dw 0
FirstDataSector:
    dw 0

times 510-($-$$) db 0                       ; Pad the boot record
dw 0xaa55                                   ; Boot signature

read_buf:                                   ; Read buffer at 0x7e00