/*
    Platform-agnostic macros
*/

/**
    Assists defining externally linkable statics and functions

    *This is mostly a vestigial feature, though it might see an increase
    in internal use in the far future.*

    # Example use
    A potential application for `relocate!` is relocating statics
    and constants to a specific section in an executable or linkable
    library:
    ```rust
        relocate! {
            pub static EMPTY_BUF: [u8; 1024] = [0u8; 1024];
                => ".bss";
        }
    ```

    A more common use-case is relocating function ("vectors")
    to a defined location in an executable:
    ```rust
        relocate! {
            pub fn in_b(port: u8) -> u8 {
                ...
            } => ".hal";

            pub fn out_b(port: u8, val: u8) {
                ...
            } => ".hal";
        }
    ```
*/
#[macro_export]
macro_rules! relocate {
    // Function with args and return type
    (
        $vis:vis fn $name:ident ( $($arg:ident : $arg_ty:ty),* ) -> $ret:ty $body:block
        => $section:literal;
        $($rest:tt)*
    ) => {
        #[unsafe(no_mangle)]
        #[unsafe(link_section = $section)]
        #[inline(never)]
        $vis extern "C" fn $name($($arg: $arg_ty),*) -> $ret $body

        relocate! { $($rest)* }
    };

    // Function with no args and return type
    (
        $vis:vis fn $name:ident () -> $ret:ty $body:block
        => $section:literal;
        $($rest:tt)*
    ) => {
        #[unsafe(no_mangle)]
        #[unsafe(link_section = $section)]
        #[inline(never)]
        $vis extern "C" fn $name() -> $ret $body

        relocate! { $($rest)* }
    };

    // Function with no args
    (
        $vis:vis fn $name:ident() $body:block
        => $section:literal;
        $($rest:tt)*
    ) => {
        #[unsafe(no_mangle)]
        #[unsafe(link_section = $section)]
        #[inline(never)]
        $vis extern "C" fn $name() $body

        relocate! { $($rest)* }
    };

    // Function with args
    (
        $vis:vis fn $name:ident ( $($arg:ident : $arg_ty:ty),* ) $body:block
        => $section:literal;
        $($rest:tt)*
    ) => {
        #[unsafe(no_mangle)]
        #[unsafe(link_section = $section)]
        #[inline(never)]
        $vis extern "C" fn $name($($arg : $arg_ty),*) $body

        relocate! { $($rest)* }
    };

    // `static` item
    (
        $vis:vis static $name:ident : $ty:ty = $value:expr;
        => $section:literal;
        $($rest:tt)*
    ) => {
        #[used]
        #[unsafe(no_mangle)]
        #[unsafe(link_section = $section)]
        $vis static $name: $ty = $value;

        relocate! { $($rest)* }
    };

    // `const` item
    (
        $vis:vis const $name:ident : $ty:ty = $value:expr;
        => $section:literal;
        $($rest:tt)*
    ) => {
        #[used]
        #[unsafe(no_mangle)]
        #[unsafe(link_section = $section)]
        $vis const $name: $ty = $value;

        relocate! { $($rest)* }
    };

    // Termination case
    () => {};
}
