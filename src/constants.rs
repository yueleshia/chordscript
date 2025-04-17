//run: cargo test -- --nocapture

macro_rules! map {
    ($arg:ident : $type:ty |> $i:ident in $from:literal .. $till:expr => {
        $( $loop_body:stmt );*
    }) => {
        {
            const fn for_loop(mut $arg : $type, $i: usize) -> $type {
                if $i < $till {
                    $( $loop_body );*
                    for_loop($arg, $i + 1)
                } else {
                    $arg
                }
            }
            for_loop($arg, $from)
        }
    }
}


// https://www.unicode.org/Public/UCD/latest/ucd/PropList.txt
// These contain no semantic meaning in head
pub const WHITESPACE: [char; 25] = [
    '\u{0009}', '\u{000a}', '\u{000b}', '\u{000c}', '\u{000d}', '\u{0020}', '\u{0085}', '\u{00a0}',
    '\u{1680}', '\u{2000}', '\u{2001}', '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}',
    '\u{2007}', '\u{2008}', '\u{2009}', '\u{200a}', '\u{2028}', '\u{2029}', '\u{202f}', '\u{205f}',
    '\u{3000}',
];

const WHITESPACE_LEN: usize = WHITESPACE.len();
const SEPARATOR_LEN: usize = WHITESPACE_LEN + 1;
pub const SEPARATOR: [char; SEPARATOR_LEN] = {
    let base = [' '; SEPARATOR_LEN];
    let mut base = map!(
        base: [char; SEPARATOR_LEN]
        |> i in 0..WHITESPACE_LEN => { base[i] = WHITESPACE[i] }
    );
    // Add these
    base[25] = '+';
    base
};

#[test]
fn check_keys_are_correct() {
    assert!(WHITESPACE.iter().all(|c| c.is_whitespace()));
    assert_eq!(&WHITESPACE, &SEPARATOR[0..WHITESPACE.len()]);
    assert!(KEYCODES.iter()
        .chain(MODIFIERS.iter())
        .all(|k| k.len() <= KEYSTR_UTF8_MAX_LEN));
    assert!(KEYCODES.iter().chain(MODIFIERS.iter())
        .any(|k| k.len() == KEYSTR_UTF8_MAX_LEN));

    // Make sure they do not intersect
    let mut combined = KEYCODES.iter().chain(MODIFIERS.iter()).collect::<Vec<_>>();
    combined.sort_unstable();
    let len = combined.len();
    combined.dedup();
    assert_eq!(len, combined.len(), "Some keys are specified both in KEYCODES and MODIFIERS");
}

pub const MODIFIERS: [&str; 4] = [
    "alt",
    "ctrl",
    "shift",
    "super",
];

pub const KEYCODES: [&str; 39] = [
    "0",
    "1",
    "2",
    "3",
    "4",
    "5",
    "6",
    "7",
    "8",
    "9",
    "a",
    "b",
    "c",
    "d",
    "e",
    "f",
    "g",
    "h",
    "i",
    "j",
    "k",
    "l",
    "m",
    "n",
    "o",
    "p",
    "q",
    "r",
    "s",
    "t",
    "u",
    "v",
    "w",
    "x",
    "y",
    "z",
    "Comma",
    "Space",
    "Return",
];

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

#[test]
fn is_keycodes_sorted_and_unique() {
    // Check if KEYCODES is sorted
    let mut sorted = KEYCODES.clone();
    sorted.sort_by(|a, b| if a.len() == b.len() {
        a.cmp(b)
    } else {
        a.len().cmp(&b.len())
    });
    assert_eq!(KEYCODES, sorted);

    // Check for duplicates
    let mut as_sorted_vec = sorted.to_vec();
    as_sorted_vec.dedup();
    assert!(as_sorted_vec.len() == sorted.len(), "KEYCODES has a duplicated key");
}
