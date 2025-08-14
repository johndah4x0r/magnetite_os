# magnetite_os/kern
The heart of the `magnetite_os` project - an attempt to write
the kernel in pure Rust. 

Writing a kernel without using glue code in C or assembly is
next to impossible, as there are platform-specific routines that 
are either too cumbersome to implement in high-level code, or are
outright impossible (such as interrupt handlers and certain
parts of paging routines).

## Testing
TODO

Consult `magnetite_os/README.md` for more information regarding
test builds and live inspection runs.

## Rationale
TODO
(probably explained in the preamble above...)

## Key concepts & terminology
TODO
(certain aspects of the preamble should be clearer
at this point)
