//run: cargo test -- --nocapture
// Here we use // std::mem::transmutate;

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
                    let base = base + $str.len();
                )*
                substr
            };
            unsafe { ::std::mem::transmute::<&[u8], &str>(&JOINED) }
        };
    };
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
