; magnetite_os - boot/src/defs.asm
; A definitions file
;
; No methods should be defined in this file,
; as we're merely storing definitions here

; Low Memory is used as follows:
; * 0x00000 .. 0x00fff                                      (do not touch)
; * 0x01000 .. 0x07aff                                      stack @ 0x7b00
; * 0x07b?? .. 0x07bff                                      stack underflow limit (used by stack frame, not enforced)
; * 0x07c00 .. 0x07dff                                      boot record (512 B)
; * 0x07e00 .. 0x08bff                                      read buffer (4 kB, accept overrun in initial stage)
; * 0x08e00 .. 0x08fff                                      buffer overflow limit (not enforced)
; Addresses relative to `ADDR_S2_LDR := 0x09000`
; are as follows (increments go left-to-right,
; top-right-to-bottom-left):
; * + 0 .. + _sizeof_s2_ldr                                 Stage-2 loader (dynamically sized; see `/link_boot1.ld`)
; * + E820_DESC_ADDR .. + n * SIZEOF_E820_ENTRY             E820 map (dynamically sized)
; * + OFFSET_PTS .. + SIZEOF_PTS                            Paging structures (may expand in the future)          

; The limit regions are kind of like the double solid-yellow
; lines on a mountain road; cross them only if you feel like
; dying in a ball of fire.

; Definitions for 'boot/src/asm/vbr.asm'
; and 'boot/src/asm/stub16.asm'
START_VECTOR                equ 0x7c00                      ; Start vector
SIZEOF_MAGIC                equ 3                           ; Size of magic numbers
OEM_LABEL                   equ START_VECTOR + SIZEOF_MAGIC ; Pointer to OEM label

INIT_STACK                  equ 0x7b00                      ; Initial value for SP

ADDR_S2_LDR                 equ 0x9000                      ; Pointer to second-stage loader

E820_ENTRIES                equ 1024                        ; maximum number of E820 map entries
SIZEOF_E820_ENTRY           equ 24                          ; size of a long E820 entry


E820_DESC_ADDR              equ 0                           ; offset for address within the E820 map descriptor
                                                            ; (location of the E820 map + descriptor relative
                                                            ; to the end of the stage-2 loader binary)
E820_DESC_SIZE              equ E820_DESC_ADDR + 8          ; offset for entry count within the E820 map descriptor
E820_DESC_END               equ E820_DESC_ADDR + 16         ; offset for the contents of the E820 map

SIZEOF_RDENTRY              equ 32                          ; Size of RDE
SIZEOF_83NAME               equ 11                          ; Size of 8.3 name

FRAME                       equ -20                         ; Start of generic frame
ROOT_DIR_SECTORS            equ FRAME + 0                   ; Offset for RDS count
FIRST_FAT_SECTOR_LOW        equ FRAME + 2                   ; Offset for FAT LBA low word
FIRST_FAT_SECTOR_HIGH       equ FRAME + 4                   ; Offset for FAT LBA high word
FIRST_RD_SECTOR_LOW         equ FRAME + 6                   ; Offset for RD LBA low word
FIRST_RD_SECTOR_HIGH        equ FRAME + 8                   ; Offset for RD LBA high word
FIRST_DATA_SECTOR_LOW       equ FRAME + 10                  ; Offset for data LBA low word
FIRST_DATA_SECTOR_HIGH      equ FRAME + 12                  ; Offset for data LBA high word
LAST_ACCESSED_LOW           equ FRAME + 14                  ; Offset for previous LBA low word
LAST_ACCESSED_HIGH          equ FRAME + 16                  ; Offset for previous LBA high word
BOOT_DEV                    equ FRAME + 18                  ; Boot device number

DAP_FRAME                   equ FRAME - 16                  ; Start of DAP frame
DAP_SIZE                    equ DAP_FRAME + 0               ; Offset for DAP size
DAP_NUM_SECTORS             equ DAP_FRAME + 2               ; Offset for sector count
DAP_BUF_OFFSET              equ DAP_FRAME + 4               ; Offset for buffer offset
DAP_BUF_SEGMENT             equ DAP_FRAME + 6               ; Offset for buffer segment
DAP_LBA_LOW                 equ DAP_FRAME + 8               ; Offset for LBA low word
DAP_LBA_MID_1               equ DAP_FRAME + 10              ; Offset for LBA low middle word
DAP_LBA_MID_2               equ DAP_FRAME + 12              ; Offset for LBA high middle word
DAP_LBA_HIGH                equ DAP_FRAME + 14              ; Offset for LBA high word

LOWEST_FRAME                equ DAP_FRAME
GUARD_SIZE                  equ 2                           ; Size of guard

ALLOC_SIZE                  equ -LOWEST_FRAME + GUARD_SIZE  ; Size of allocated frame
INIT_FRAME                  equ INIT_STACK + ALLOC_SIZE     ; Initial value for BP

RDE_FIRST_CLUSTER           equ 26                          ; Offset for cluster ID 0

SCREEN_WIDTH                equ 800                         ; Requested screen width
SCREEN_HEIGHT               equ 600                         ; Requested screen height

; Definitions for 'boot/src/stub32.asm'
NULL                equ 0               ; Null pointer

; Flags for later use
ID_EFLAGS           equ 1 << 21         ; EFLAGS ID bit
EXT_CPUID           equ 1 << 31         ; CPUID extensions
FEAT_CPUID          equ (1 << 31) | 1   ; CPUID extended features
LM_EDX_CPUID        equ 1 << 29         ; Long mode bit
NO_PAGING           equ 0x7fffffff      ; No 32-bit paging

PAE_ENABLE          equ 1 << 5          ; Enable PAE in CR4
PG_ENABLE           equ 1 << 31         ; Enable paging in CR0


; Paging hierarchy layout
; - for starters, offset for bootstrap paging structures
SIZEOF_PT           equ 1 << 12         ; Sets page table size to 4 kiB

; - location of PML4 table (master hierarchy)
; (offset relative to page boundary-aligned base
; just past the E820 map)
OFFSET_PTS          equ 0
OFFSET_PML4         equ OFFSET_PTS

; - location of PDP table (huge)
OFFSET_PDPT         equ OFFSET_PML4 + SIZEOF_PT

; - location of page directory table (large)
OFFSET_PDT          equ OFFSET_PDPT + SIZEOF_PT

; - location of page table(s) (standard)
OFFSET_PT           equ OFFSET_PDT + SIZEOF_PT

; - size of the bootstrap paging hierarchy
SIZEOF_PTS          equ OFFSET_PT + SIZEOF_PT - OFFSET_PML4

; Page masks and flags
PT_ADDR_MASK        equ 0xffffffffff000 ; Mask to align addresses to 4 kiB
PT_PRESENT          equ 1               ; Marks page as present
PT_READWRITE        equ 2               ; Marks page as R/W
PT_PAGESIZE         equ 128             ; Marks page as large/huge (if needed)

SIZEOF_PAGE         equ 1 << 12         ; Sets page size to 4 kiB (normal pages)

EFER_MSR            equ 0xC0000080      ; EFER MSR address
EFER_LME            equ 0x100           ; EFER IA-32e set bit

ENTRIES_PER_PT      equ 512             ; Entries per page table
SIZEOF_PT_ENTRY     equ 8               ; Size of PT entry (64 bits)

; GDT bits
; - access bits
SEG_PRESENT         equ 1 << 7          ; Present bit
SEG_NOT_SYS         equ 1 << 4          ; Non-system segment
SEG_EXEC            equ 1 << 3          ; Executable segment
SEG_RW              equ 1 << 1          ; Read-write segment

; - flags bits
SEG_GRAN_4K         equ 1 << 7          ; Page granularity
SEG_SZ_32           equ 1 << 6          ; 32-bit size
SEG_LONG_MODE       equ 1 << 5          ; Long mode segment
