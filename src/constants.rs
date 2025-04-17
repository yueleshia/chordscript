//run: cargo test -- --nocapture

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
    let mut base = [' '; SEPARATOR_LEN];

    const fn copy_whitespace_into(mut base: [char; SEPARATOR_LEN], i: usize) -> [char; 26] {
        if i < WHITESPACE_LEN {
            base[i] = WHITESPACE[i];
            copy_whitespace_into(base, i + 1)
        } else {
            base
        }
    }
    copy_whitespace_into(base, 0);

    // Add these
    base[25] = '+';
    base
};

#[test]
fn check_whitespace() {
    for c in &WHITESPACE {
        assert!(c.is_whitespace());
    }
}


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
