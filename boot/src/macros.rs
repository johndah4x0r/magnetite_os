/*
    The macros here are mostly auto-generated.
    Expect therefore unseen inconsistencies.
*/

/*
    A macro that aids in defining externally linkable
    functions and statics using a relatively simple
    Rust-like syntax
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
