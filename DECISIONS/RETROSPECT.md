# Past decisions
Past decisions that were made before the decisions record was
established can be found here. The decisions may not have an
exact date of introduction, so the retrospective descriptions
are instead labeled by the date they were authored.

(interactive - up one level: [`magnetite_os/`](../))


## Retrospect 2025-08-30, part 2

### Title
Implement self-contained volume boot record with file system
parsing capability

### Introduction date
- Work began **2025-08-18** (`old-vbr`...`7c2bd60`)
- Finalized **2025-08-27** (`new-vbr` = `6e95d0d`)

### Status
Sustained

### Rationale
Normally, loading raw sectors appended to the volume boot record
(VBR) would be enough, but if real-world support is desired, then
the VBR should be capable of parsing file systems and loading a
second-stage loader from indeterminate locations.

From a more personal angle, implementing a self-contained VBR would
prove to be a productive challenge: making a boot sector program
both space-efficient and resillient.

### Consequence
- **The decision is sustained.**
- The VBR no longer attempts to escape real mode, and instead focuses
on loading a second-stage loader from a FAT16 volume using BIOS disk
I/O services and emulated 32-bit arithmetic.


## Retrospect 2025-08-30, part 1

### Title
Implement Relocatable HAL with ABI+API Contracts

### Introduction date
*Unknown; gradual*

### Status
Reversed (through curating)

### Rationale
The initial thought process was that the bootloader and the kernel
should be able to reuse the same hardware drivers *not just* at the
source level, but *also* at the binary level.

It was envisioned that the second-stage loader should have a HAL and
a vector table (VT) packaged together with it. In addition, the 
HAL and VT should be relocatable, so that they can still be used by
the kernel after copying them into kernel memory: relocatability was,
and still is, a key concern, as some memory accesses are still
absolute (as opposed to relative).

It wasn't until much later that key concerns became evident, among
them being ABI compatibility and HAL extensibility.
- Programs wishing to use the bootloader's HAL *must* share the same
definitions for the dispatch table and type-specific methods.
- Extensibility at the memory level is virtually impossible to
implement without sacrificing type safety, as legal implementations
of linked lists (which an extensible HAL might be based on) require all nodes in a list to be of the same type (owing to Rust's type
safety rules).

In addition, the kernel would be loading its own drivers later in
the boot process, so sharing a minimal HAL would be redundant.

As the core kernel logic should in principle be platform-agnostic,
it follows that the kernel executable should be self-sufficient
whenever possible. Dynamically linking a HAL using platform-specific
contracts would go against both principles.

### Consequence
- The commit history for the `main` and `experimental` branches was
curated using `git rebase --commiter-date-is-author-date -i <hash>`,
so that the relocatable HAL appears to have *never* been implemented.
- The non-curated history has been preserved in the `main.old.2025-08-30`
branch (using `git checkout main -b main.old.2025-08-30` before `main`
was curated).
- HAL will be implemented as a source-level shared resource (outlined
in [Decision 2025-08-30, part 2](./DECISION_2025-08-30_2.md).
