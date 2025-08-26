; magnetite_os - boot/src/defs.asm
; A definitions file
;
; No methods should be defined in this file,
; as we're merely storing definitions here

; Low Memory is used as follows:
; * 0x00500-0x07af0 - stack (to be relocated)
; * 0x07b00-0x07bff - guard region (not enforced)
; * 0x07c00-0x07dff - boot record (512 B)
; * 0x07e00-0x08bff - read buffer (4 kB, accept overrun in initial stage)
; * 0x09000-0xefff  - E820 memory map (24 kB)
; * 0x0f000-0x10fff - Second stage loader (8 kB)
; * 0x11000-0x1ffff - minimum usable memory (128 kB)
; * 0x20000-0x7ffff - maximum usable memory (>128 kB)

; Definitions for 'boot/src/vbr.asm'
; and 'boot/src/stub16.asm'
START_VECTOR                equ 0x7c00                      ; Start vector
SIZEOF_MAGIC                equ 3                           ; Size of magic numbers
OEM_LABEL                   equ START_VECTOR + SIZEOF_MAGIC ; Pointer to OEM label

ADDR_S2_LDR                 equ 0xf000                      ; Pointer to second-stage loader
ADDR_E820_MAP               equ 0x9000                      ; Pointer to E820 map

E820_ENTRIES                equ 1024                        ; E820 map entries

SIZEOF_RDENTRY              equ 32                          ; Size of RDE
SIZEOF_83NAME               equ 11                          ; Size of 8.3 name

FRAME                       equ -18                         ; Start of generic frame
ROOT_DIR_SECTORS            equ FRAME + 0                   ; Offset for RDS count
FIRST_FAT_SECTOR_LOW        equ FRAME + 2                   ; Offset for FAT LBA low word
FIRST_FAT_SECTOR_HIGH       equ FRAME + 4                   ; Offset for FAT LBA high word
FIRST_RD_SECTOR_LOW         equ FRAME + 6                   ; Offset for RD LBA low word
FIRST_RD_SECTOR_HIGH        equ FRAME + 8                   ; Offset for RD LBA high word
FIRST_DATA_SECTOR_LOW       equ FRAME + 10                  ; Offset for data LBA low word
FIRST_DATA_SECTOR_HIGH      equ FRAME + 12                  ; Offset for data LBA high word
LAST_ACCESSED_LOW           equ FRAME + 14                  ; Offset for previous LBA low word
LAST_ACCESSED_HIGH          equ FRAME + 16                  ; Offset for previous LBA high word

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

RDE_FIRST_CLUSTER           equ 26                          ; Offset for cluster ID 0


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


; Page hierarchy layout
PML4T_ADDR          equ 0x1000          ; Location of PML4 table (master hierarchy)
PDPT_ADDR           equ 0x2000          ; Location of PDP table (huge)
PDT_ADDR            equ 0x3000          ; Location of page directory table (large)
PT_ADDR             equ 0x4000          ; Location of page table (standard)


; Page masks and flags
PT_ADDR_MASK        equ 0xffffffffff000 ; Mask to align addresses to 4 kiB
PT_PRESENT          equ 1               ; Marks page as present
PT_READWRITE        equ 2               ; Marks page as R/W
PT_PAGESIZE         equ 128             ; Marks page as large/huge (if needed)

SIZEOF_PAGE         equ 1 << 12         ; Sets page size to 4 kiB (normal pages)
SIZEOF_PT           equ 1 << 12         ; Sets page table size to 4 kiB

ENTRIES_PER_PT      equ 512             ; Entries per page table
SIZEOF_PT_ENTRY     equ 8               ; Size of PT entry (64 bits)

EFER_MSR            equ 0xC0000080      ; EFER MSR address
EFER_LME            equ 0x100           ; EFER IA-32e set bit


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
