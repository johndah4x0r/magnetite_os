/*
    The macros here are mostly auto-generated.
    Expect therefore unseen inconsistencies.
*/

// Thise one uses a custom syntax, so there
// should be no surprises here...
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

// Instantiate access-controlled HAL VT
//
// The rules here are *slightly* different to idiomatic Rust
// - $struct_name does NOT actually set the type for $instance,
//   as it will always be 'HalVtableAC'. We only require
//   $struct_name so that we can use an unused name
#[macro_export]
macro_rules! hal_vt_instance {
    // Enforce public visibility
    (
        $(#[$meta:meta])*
        pub static $instance:ident : $struct_name:ident = {
            $(
                $field:ident : $ty:ty = $val:expr
            ),* $(,)?
        }; => $section:literal;
    ) => {
        // Define the struct type
        #[repr(C)]
        pub struct $struct_name {
            $(
                pub $field: $crate::structs::wrappers::HalVtableEntry<$ty>,
            )*
        }

        impl $struct_name {
            pub const fn new(
                $($field: $ty),*
            ) -> Self {
                $struct_name {
                    $(
                        $field: $crate::structs::wrappers::HalVtableEntry::new($field),
                    )*
                }
            }
        }

        // Implement marker trait
        impl $crate::structs::HalVectorTable for $struct_name {}

        // Define the instance
        $(#[$meta])*
        #[unsafe(no_mangle)]
        #[unsafe(link_section = $section)]
        pub static $instance: $crate::structs::HalVtableAC<$struct_name> = $crate::structs::HalVtableAC::new(
            core::sync::atomic::AtomicIsize::new(0),
            $struct_name::new($($val),*),
        );
    };
}
