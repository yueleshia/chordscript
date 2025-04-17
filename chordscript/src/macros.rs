//run: cargo test -- --nocapture
// run: cargo run
// Here we use std::mem::transmutate;

// No dependencies here

#[macro_export]
macro_rules! map {
    // We have to unroll the loop because we hit the stack limit too quickly
    ($first:ident : $type:ty $(, $other_arg:ident : $other_type:ty)*
        |> $i:ident in $from:literal .. $till:expr => $loop_body:expr
    ) => { {
        const fn for_loop(
            mut $first : $type,
            $( $other_arg : $other_type, )*
            $i: usize,
            till: usize
        ) -> $type {
            // assert!(+ 10 is the same as 1-9 for the unrolling)
            let next_base = $i + 10;
            $crate::map!(@unroll for_loop($first, $( $other_arg, )* next_base, till),
                $first $i < till 1 2 3 4 5 6 7 8 9
                => $loop_body
            )
        }
        for_loop($first, $( $other_arg, )* $from, $till)
    } };

    // This is basically what it would look like not unrolled
    (@unroll $loop:expr, $mut:ident $i:ident < $limit:ident => $body:expr) => {
        if $i < $limit {
            $body;
            $loop
        } else {
            $mut
        }
    };

    // $_ eats one of the entries
    (@unroll $loop:expr, $mut:ident $i:ident < $limit:ident
        $_:literal $( $j:literal )* => $body:expr
    ) => {
        if $i < $limit {
            $body;
            let $i = $i + 1;
            $crate::map!(@unroll $loop, $mut $i < $limit $( $j )* => $body)
        } else {
            $mut
        }
    };
}

#[macro_export]
macro_rules! const_concat {
    (const $var:ident = $( $str:expr )=>*) => {
        pub const $var: &str = {
            const SIZE: usize = 0 $( + $str.len() )*;
            const JOINED: [u8; SIZE] = {
                let substr = [0; SIZE];
                let base = 0;
                $(
                    let raw_str = $str.as_bytes();
                    let substr = $crate::map!(
                        substr: [u8; SIZE], base: usize, raw_str: &[u8]
                        |> i in 0..$str.len() => {
                            substr[base + i] = raw_str[i]
                        }
                    );
                    #[allow(unused_variables)]
                    let base = base + $str.len();
                )*
                substr
            };
            unsafe { ::std::mem::transmute::<&[u8], &str>(&JOINED) }
        };
    };
}

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



#[test]
fn const_concat() {
    // Test loop unrolling is working
    let test = [0u8; 53];
    let test = map!(test: [u8; 53] |> i in 0..37 => { test[i] = i as u8 + 1 });
    let mut target = [0u8; 53];
    for i in 0..37 {
        target[i] = i as u8 + 1;
    }
    assert_eq!(target, test);

    // Concat
    const FIRST: &'static str = "The quick brown fox jumps over";
    const_concat!(const ASDF = FIRST => " the lazy dog");
    assert_eq!("The quick brown fox jumps over the lazy dog", ASDF);
}
