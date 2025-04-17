//run: cargo test -- --nocapture
// run: cargo run
// Here we use std::mem::transmutate;

// No dependencies here

// This macro is for ergonomics, capacity and str can be specified on one line
// This then calculates total capacity, allocates, then pushes
#[macro_export]
macro_rules! precalculate_capacity_and_build {
    ($this:ident, $buffer:ident {
        $( $init:stmt; )*
    } {
        $( $stmts:tt )*
    }) => {
        fn string_len(&$this) -> usize {
            $( $init )*
            let capacity = precalculate_capacity_and_build!(@size $($stmts)*);
            capacity
        }
        fn push_string_into(&$this, $buffer: &mut String) {
            //#[cfg(debug_assertions)]
            //debug_assert!({ $this.to_string_custom(); true });
            $( $init )*
            precalculate_capacity_and_build!($buffer @push $($stmts)*);
        }

        #[cfg(debug_assertions)]
        fn to_string_custom(&$this) -> String {
            $( $init )*
            let capacity = $this.string_len();
            let mut owner = String::with_capacity(capacity);
            let $buffer = &mut owner;
            precalculate_capacity_and_build!($buffer @push $($stmts)*);
            debug_assert_eq!(capacity, $buffer.len(),
                "Pre-calculated capacity is incorrect.");
            owner
        }
    };

    (@size $size:expr => $push:expr; $($rest:tt)*) => {
        $size + precalculate_capacity_and_build!(@size $($rest)*)
    };
    (@size $str:expr; $($rest:tt)*) => {
        $str.len() + precalculate_capacity_and_build!(@size $($rest)*)
    };
    (@size) => { 0 };

    ($buffer:ident @push $size:expr => $push:expr; $($rest:tt)*) => {
        $push;
        precalculate_capacity_and_build!($buffer @push $($rest)*);
    };
    ($buffer:ident @push $str:literal; $($rest:tt)*) => {
        $buffer.push_str($str);
        precalculate_capacity_and_build!($buffer @push $($rest)*);
    };
    ($buffer:ident @push) => { 0 };
}

// A way specify length of what is pushed and do the pushing side-by-side
#[macro_export]
macro_rules! sidebyside_len_and_push {
    (
        $(! $( $prefix:ident )+ !)? $len:ident $(<$( $len_lt:lifetime ),*>)?,
        $push_into:ident $(<$($push_lt:lifetime),*>)?
            ($self:ident : $ty1:ty, $extra:ident : $ty2:ty, $buffer:ident: $filestr:lifetime)
        {
            $( $init:stmt; )*
        } {
            $( $stmts:tt )*
        }
    ) => {
        $( $( $prefix )* )? fn $len $(<$($len_lt),*>)? ($self: $ty1, $extra: $ty2) -> usize {
            $( $init )*
            sidebyside_len_and_push!(@size $($stmts)*)
        }
        fn $push_into $(<$($push_lt),*>)? ($self: $ty1, $extra: $ty2, $buffer: &mut Vec<&$filestr str>) {
            //#[cfg(debug_assertions)]
            //debug_assert!({ $this.to_string_custom(); true });
            $( $init )*
            sidebyside_len_and_push!($buffer @push $($stmts)*);
        }

    };

    // We support two styles of specifying a line either
    //    {} => {};
    //    {};

    // The rest of this is using the TT-mucher pattern
    (@size $size:expr => $push:expr; $($rest:tt)*) => {
        $size + sidebyside_len_and_push!(@size $($rest)*)
    };
    // Additionally we support
    (@size $str:expr; $($rest:tt)*) => {
        1 + sidebyside_len_and_push!(@size $($rest)*)
    };
    (@size) => { 0 };

    ($buffer:ident @push $size:expr => $push:expr; $($rest:tt)*) => {
        $push;
        sidebyside_len_and_push!($buffer @push $($rest)*);
    };
    ($buffer:ident @push $str:literal; $($rest:tt)*) => {
        $buffer.push($str);
        sidebyside_len_and_push!($buffer @push $($rest)*);
    };
    ($buffer:ident @push) => { 0 };
}

// @TODO: Constants can probably use this
#[macro_export]
macro_rules! pick {
    (1 => $me:expr $(=> $_:expr)*          ) => { $me };
    (2 => $_1:expr => $me:expr => $_:expr  ) => { $me };
    (3 => $_1:expr => $_2:expr => $me:expr ) => { $me };
}

#[macro_export]
macro_rules! array_index_by_enum {
    ($ROW_COUNT:ident : usize
    pub enum $Enum:ident {
        $( $Variant:ident $( => $val:expr )* , )*
    } $( $rest:tt )*) => {
        #[derive(Debug)]
        #[repr(usize)]
        pub enum $Enum {
            $( $Variant, )*
        }
        impl $Enum {
            #[allow(dead_code)]
            pub const fn id(&self) -> usize {
                unsafe { *(self as *const Self as *const usize) }
            }
        }

        const $ROW_COUNT: usize = 0 $( + { let _ = $Enum::$Variant; 1 } )*;
        array_index_by_enum!($( $( => $val)*, )* = $ROW_COUNT $($rest)*);
    };

    ($( $(=> $val:expr)*, )* = $len:ident => $n:tt pub const $VEC:ident : [$ty:ty]
        $( $rest:tt )*
    ) => {
        pub const $VEC: [$ty; $len] = [$( $crate::pick!($n $(=> $val )*), )*];
        array_index_by_enum!($( $(=> $val)*, )* = $len $( $rest )*);
    };

    ($( $_:tt)*) => {}; // End tt-muncher
}
