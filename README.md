## Organization & build flow
This project is currently divided into two sub-projects (interactive - links are clickable):
- [`magnetite_os/boot`](boot/) - **(WIP)** custom legacy bootloader written 
    in assembler and Rust 
- [`magnetite_os/kern`](kern/) - **(planned)** kernel written in Rust
- [`magnetite_os/common`](common/) - **(WIP)** common definitions
- [`magnetite_os/DECISIONS`](DECISIONS/) - decisions record

## Toolchain requirements
This project primarily uses 
- `make` to keep track of progress, resolve dependencies,
and orchestrate component builds,
- `cargo` (from the Rust `nightly` toolchain) to compile Rust sources
- `nasm` to compile x86 assembly sources,
- `ld` (from `binutils`) to link binaries with custom layouts, 
- `mkfs.fat` and `mcopy` (from `dosfstools` and `mtools`) to generate
FAT16 disk images containing system files, and
- `bochs` to run the project in a virtual machine

## Testing
To build and debug the bootloader, simply run
```bash
make debug_boot
```

## Rationale
TODO

## Key concepts & terminology
Specific concepts and terminology are more appropriately explained
in their respective contexts: 
- boot-related breakdown in `magnetite_os/boot/README.md`, and
- kernel-related breakdown in `magnetite_os/kern/README.md`

## Resources
This project, like many other hobby OS development projects, would be
next to impossible without reading into relevant literature.

A good place to look up basics and details would be 
the [OSDev wiki page](https://wiki.osdev.org).
