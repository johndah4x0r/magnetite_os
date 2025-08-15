## Organization & build flow
This project is currently divided into two sub-projects (interactive - links are clickable):
- [`magnetite_os/boot`](boot/) - **(WIP)** custom legacy bootloader written 
    in assembler and Rust 
- [`magnetite_os/kern`](kern/) - **(planned)** kernel written in Rust

## Testing
TODO

To build and debug the bootloader, simply run
```bash
make debug_boot
```

This project primarily uses `make` to keep track of progress,
resolve dependencies and orchestrate component builds.

Since this project uses Rust and x86 assembly, it would 
be prudent to install `nasm`, GNU `binutils` and the 
Rust toolchain.

## Rationale
TODO

I have no idea why I even bothered lifting a finger...

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