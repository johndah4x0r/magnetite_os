/*
    General platform-agnostic structures

    The structures defined in the submodules are either fully
    or partially binary-compatible with the contained types
    (if they contain anything at all).
*/

// Use `include!` macro
use core::include;

// Volatile wrapper type
include!("volatile.rs");

// Array-like fat pointer type
include!("array_like.rs");

// fat UART port type
include!("uart.rs");
