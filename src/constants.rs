//run: cargo test -- --nocapture

use crate::map;

// https://www.unicode.org/Public/UCD/latest/ucd/PropList.txt
// These contain no semantic meaning in head
pub const WHITESPACE: [char; 25] = [
    '\u{0009}', '\u{000a}', '\u{000b}', '\u{000c}', '\u{000d}', '\u{0020}', '\u{0085}', '\u{00a0}',
    '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}',
    '\u{2007}', '\u{2008}', '\u{2009}', '\u{200a}', '\u{2028}', '\u{2029}', '\u{202f}', '\u{205f}',
    '\u{3000}',
];

const SEPARATOR_LEN: usize = WHITESPACE.len() + 1;
pub const SEPARATOR: [char; SEPARATOR_LEN] = {
    let base = [' '; SEPARATOR_LEN];
    let mut base = map!(
        base: [char; SEPARATOR_LEN]
        |> i in 0..WHITESPACE.len() => { base[i] = WHITESPACE[i] }
    );
    // Add these
    base[25] = '+';
    base
};

// The only other way I can think of building 'AVAILABLE_KEYS' without using
// a macro is #![feature(const_str_from_utf8_unchecked)]
// See: https://github.com/rust-lang/rust/issues/75196
macro_rules! build_available_keys {
    ($( pub const $var:ident : $type:ty = [$( $val:literal, )*]; )*) => {
        $( pub const $var: $type = [$( $val, )*]; )*
        pub const AVAILABLE_KEYS: &str =
            build_available_keys!(@join $( $( $val, )* )*);
    };

    (@join $first:literal, $( $val:literal, )*) => {
        concat!($first, $(", ", $val, )*)
    };
}

build_available_keys! {
    pub const MODIFIERS: [&str; 4] = ["alt", "ctrl", "shift", "super",];
    pub const KEYCODES: [&str; 39] = [
        "0", "1", "2", "3", "4", "5", "6", "7", "8", "9",
        "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m",
        "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z",
        "Comma",
        "Space",
        "Return",
    ];
}

pub const KEYSTR_UTF8_MAX_LEN: usize = {
    let max_len = 0;
    let max_len = map!(
        max_len: usize
        |> i in 0..KEYCODES.len() => {
            if KEYCODES[i].len() > max_len {
                max_len = KEYCODES[i].len()
            }
        }
    );
    map!(
        max_len: usize
        |> i in 0..MODIFIERS.len() => {
            if MODIFIERS[i].len() > max_len {
                max_len = MODIFIERS[i].len()
            }
        }
    )
};

// Many tests to check for human input error
#[cfg(test)]
mod test {
    use super::*;

    fn assert_is_unique<T: Ord>(mut input: Vec<T>, msg: &str) {
        let before_sort_len = input.len();
        input.sort_unstable();
        input.dedup();
        assert_eq!(before_sort_len, input.len(), "'{}' has a duplicate", msg);
    }

    #[test]
    fn arrays_are_unique_and_valid() {
        assert!(WHITESPACE.iter().all(|c| c.is_whitespace()));
        assert_eq!(&WHITESPACE, &SEPARATOR[0..WHITESPACE.len()]);

        assert_is_unique(WHITESPACE.to_vec(), stringify!(WHITESPACE));
        assert_is_unique(SEPARATOR.to_vec(), stringify!(SEPARATOR));
        assert_is_unique(KEYCODES.to_vec(), stringify!(KEYCODES));
    }

    #[test]
    fn keycodes_is_sorted() {
        // Check if KEYCODES is sorted
        let mut sorted = KEYCODES.clone();
        sorted.sort_by(|a, b| if a.len() == b.len() {
            a.cmp(b)
        } else {
            a.len().cmp(&b.len())
        });
        assert_eq!(KEYCODES, sorted);
    }

    #[test]
    fn key_and_mod_for_stats_and_overlap() {
        let mut combined = MODIFIERS.to_vec();
        combined.append(&mut KEYCODES.to_vec());
        let combined = combined; // remove mut

        assert!(combined.iter().all(|k| k.len() <= KEYSTR_UTF8_MAX_LEN));
        assert!(combined.iter().any(|k| k.len() == KEYSTR_UTF8_MAX_LEN));

        // Make sure they do not intersect
        assert_is_unique(combined.clone(), "KEYCODE' together with 'MODIFIER");

        // Make sure we built 'AVAILABLE_KEYS' correctly
        assert_eq!(combined.clone().join(", "), AVAILABLE_KEYS);
    }
}
