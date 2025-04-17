//run: cargo test -- --nocapture

////////////////////////////////////////////////////////////////////////////////
// Macros
////////////////////////////////////////////////////////////////////////////////

use crate::array_index_by_enum;

macro_rules! copy_from_slice {
    ($base:ident[{ $start:expr }..] = $from:ident) => {
        assert!($base.len() >= $from.len());
        let mut i = 0;
        loop {
            $base[$start + i] = $from[i];
            i += 1;
            if i >= $from.len() {
                break;
            }
        }
    };
}

macro_rules! const_join_str {
    (pub const $JOIN:ident: &str = $list:ident | join($raw:ident $len:ident)) => {
        const $len: usize = {
            let mut len = 0;
            let mut i = 0;
            loop {
                len += $list[i].len() + 1;
                i += 1;
                if i >= $list.len() {
                    break;
                }
            }
            len - 1
        };

        const $raw: [u8; $len] = {
            let mut temp = [' ' as u8; $len];
            let mut i = 0;
            let mut len = 0;
            loop {
                // len += ALL_KEYS[i].len() + 1;
                let slice = $list[i].as_bytes();
                copy_from_slice!(temp[{ len }..] = slice);
                len += $list[i].len() + 1;
                i += 1;
                if i < $list.len() {
                } else {
                    break;
                }
            }
            temp
        };
        pub const $JOIN: &str = unsafe { std::str::from_utf8_unchecked(&$raw) };
    };
}

////////////////////////////////////////////////////////////////////////////////
// Constants
////////////////////////////////////////////////////////////////////////////////

pub const VALID_ESCAPEES: [&str; 5] = ["\\", "|", ",", "n", "\\n"];
const_join_str!(pub const VALID_ESCAPEE_STR: &str = VALID_ESCAPEES | join(A_RAW A_LEN));

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
    let mut base = [' '; SEPARATOR_LEN];
    copy_from_slice!(base[{ 0 }..] = WHITESPACE);
    base[25] = '+';
    base
};

// Following the naming conventions of xev for the keys
array_index_by_enum! { MODIFIER_COUNT: usize
    pub enum Modifiers {
        Alt => "alt", Ctrl => "ctrl", Shift => "shift", Super => "super",
    } => 1 pub const MODIFIERS: [&str]
}
pub const MOD_UTF8_MAX_LEN: usize = fold_max_len(&MODIFIERS);

array_index_by_enum! { KEYCODE_COUNT: usize
    pub enum Keycodes {
        Comma => ",",
        Period => ".",
        Zero => "0", One => "1", Two => "2", Thre => "3", Four => "4",
        Five => "5", Six => "6", Seven => "7", Eight => "8", Nine => "9",
        A => "a", B => "b", C => "c", D => "d", E => "e", F => "f", G => "g",
        H => "h", I => "i", J => "j", K => "k", L => "l", M => "m", N => "n",
        O => "o", P => "p", Q => "q", R => "r", S => "s", T => "t", U => "u",
        V => "v", W => "w", X => "x", Y => "y", Z => "z",
        Up => "Up", Down => "Down", Left => "Left",
        Print => "Print",
        Right => "Right",
        Space => "space",
        Insert => "Insert",
        Return => "Return",
        BackSpace => "BackSpace",
        Semicolon => "semicolon",
        XF86MonBrightnessUp => "XF86MonBrightnessUp",
        XF86MonBrightnessDown => "XF86MonBrightnessDown",
    } => 1 pub const KEYCODES: [&str]
}
pub const KEY_UTF8_MAX_LEN: usize = fold_max_len(&KEYCODES);

const ALL_KEYS: [&str; MODIFIERS.len() + KEYCODES.len()] = {
    let mut base = [""; MODIFIERS.len() + KEYCODES.len()];
    copy_from_slice!(base[{ 0 }..] = MODIFIERS);
    copy_from_slice!(base[{ MODIFIERS.len() }..] = KEYCODES);
    base
};
// Join ALL_KEYS for printing an error message
const_join_str!(pub const AVAILABLE_KEYS: &str = ALL_KEYS | join(JOIN_RAW JOIN_LEN));

const fn fold_max_len(list: &[&str]) -> usize {
    let mut max_len = 0;
    let mut i = 0;
    loop {
        if max_len < list[i].len() {
            max_len = list[i].len();
        }
        i += 1;
        if i >= list.len() {
            break;
        }
    }
    max_len
}

// Many tests to check for human input error
#[cfg(test)]
mod test {
    use super::*;
    const BLACKLIST: [char; 6] = ['|', '!', '"', ';', '{', '}'];
    const KEYSTR_UTF8_MAX_LEN: usize = fold_max_len(&ALL_KEYS);

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
        assert_is_unique(MODIFIERS.to_vec(), stringify!(KEYCODES));

        assert!(!KEYCODES.iter().any(|s| s.contains(&BLACKLIST[..])));
        assert!(!MODIFIERS.iter().any(|s| s.contains(&BLACKLIST[..])));
    }

    #[test]
    fn keycodes_is_sorted() {
        // Check if KEYCODES is sorted
        let mut sorted = KEYCODES.clone();
        sorted.sort_by(|a, b| {
            if a.len() == b.len() {
                a.cmp(b)
            } else {
                a.len().cmp(&b.len())
            }
        });
        assert_eq!(sorted, KEYCODES);
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
        assert_eq!(combined.clone().join(" "), AVAILABLE_KEYS);
    }
}
