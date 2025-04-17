macro_rules! import {
    ($mod:ident::$struct:ident) => {
        mod $mod;
        pub use $mod::$struct;
    };
    ($mod:ident::{$( $struct:ident ),*}) => {
        mod $mod;
        pub use $mod::{$( $struct ),*};
    };
}

import!(i3::I3);
import!(keyspace_preview::KeyspacePreview);
import!(list_preview::{ListPreview, ListChord});

use std::cmp;
use crate::precalculate_capacity_and_build;
use crate::structs::{Chord, WithSpan};

pub trait Print {
    fn string_len(&self) -> usize;
    fn push_string_into(&self, buffer: &mut String);
    // @TODO cfg(debug) only
    fn to_string_custom(&self) -> String;
}

#[macro_export]
macro_rules! array {
    // For displaying:
    //   $prefix $list[0] $suffix
    //   $prefix $list[1] $suffix
    //   $prefix $list[2] $suffix
    //   ...
    (@len { $list:expr } |> $prefix:expr, $wrapper:ident, $suffix:literal) => {
        $list
            .iter()
            .map(|item| $prefix.len() + $wrapper(item).string_len() + $suffix.len())
            .sum::<usize>()
    };
    (@push { $list:expr }
        |> $prefix:expr, $wrapper:ident, $suffix:expr,
        |> $buffer:ident
    ) => {
        $list.iter().for_each(|item| {
            $buffer.push_str($prefix);
            $wrapper(item).push_string_into($buffer);
            $buffer.push_str($suffix);
        })
    };

    // For displaying:
    //   $list[0] $delim $list[1] $delim $list[2] ... $delim $list[n]
    (@len_join { $list:expr } |> $wrapper:ident, $delim:expr) => {{
        let mut iter = $list.iter();
        if let Some(first) = iter.next() {
            $wrapper(first).string_len()
                + iter
                    .map(|item| $wrapper(item).string_len() + $delim.len())
                    .sum::<usize>()
        } else {
            0
        }
    }};
    (@push_join { $list:expr } |> $wrapper:ident, $delim:expr, |> $buffer:ident) => {
        let mut iter = $list.iter();
        if let Some(first) = iter.next() {
            $wrapper(first).push_string_into($buffer);
            iter.for_each(|item| {
                $buffer.push_str($delim);
                $wrapper(item).push_string_into($buffer);
            })
        }
    };

    // For testing
    (@to_string { $list:expr } |> $wrapper:ident) => {{
        let mut string = String::new();
        $list
            .iter()
            .for_each(|item| $wrapper(item).push_string_into(&mut string));
        string
    }};
}

// This macro is for ergonomics, capacity and str can be specified on one line
// This then calculates total capacity, allocates, then pushes
#[macro_export]
macro_rules! precalculate_capacity_and_build {
    ($this:ident, $buffer:ident {
        $( $init:stmt; )*
    } {
        $( $size:expr => $push:expr; )*
    }) => {
        fn string_len(&$this) -> usize {
            $( $init )*
            let capacity = 0 $( + $size )*;
            capacity
        }

        fn push_string_into(&$this, $buffer: &mut String) {
            debug_assert!({ $this.to_string_custom(); true });
            $( $init )*
            $( $push; )*
        }

        //#[cfg(Debug)]
        fn to_string_custom(&$this) -> String {
            $( $init )*
            let capacity = $this.string_len();
            let mut owner = String::with_capacity(capacity);
            let $buffer = &mut owner;
            $( $push; )*
            debug_assert_eq!(capacity, $buffer.len(),
                "Pre-calculated capacity is incorrect.");
            owner
        }

    };
}

pub struct EscapedStrList<'list, 'filestr>(
    char,
    &'static [char],
    &'static [&'static str],
    &'list [WithSpan<'filestr, ()>]
);
struct EscapedStr<'list, 'filestr>(
    &'static [char],
    &'static [&'static str],
    &'list WithSpan<'filestr, ()>
);

impl<'list, 'filestr> Print for EscapedStrList<'list, 'filestr> {
    precalculate_capacity_and_build!(self, buffer {
        let Self(quote, candidates, escape, list) = self;
        debug_assert_eq!(candidates.len(), escape.len());
    } {
        quote.len_utf8() => buffer.push(*quote);
        list.iter().map(|s|
            EscapedStr(candidates, escape, s).string_len()
        ).sum::<usize>() => list.iter().for_each(|s|
            EscapedStr(candidates, escape, s).push_string_into(buffer)
        );
        quote.len_utf8() => buffer.push(*quote);
    });
}

impl<'list, 'filestr> Print for EscapedStr<'list, 'filestr> {
    precalculate_capacity_and_build!(self, buffer {
        let Self(candidates, escape, substr_span) = self;
        debug_assert_eq!(candidates.len(), escape.len());

        let mut iter = DelimSplit::new(substr_span.as_str(), candidates);
        let (_, first) = iter.next().unwrap();

        let mut char_to_str = [0u8; 4];
        let new_iter = iter.map(|(delim, s)| {
            let index = candidates.iter()
                .position(|c| c.encode_utf8(&mut char_to_str) == delim)
                .unwrap(); // This should always exist
            (&escape[index], s)
        });

    } {
        first.len() => buffer.push_str(first);
        new_iter.map(|(escaped, s)| escaped.len() + s.len()).sum::<usize>() => {
            new_iter.for_each(|(escaped_delim, s)| {
                buffer.push_str(escaped_delim);
                buffer.push_str(s);
            })
        };
    });
}

pub fn index_of_substr<'a>(source: &'a str, substr: &'a str) -> usize {
    (substr.as_ptr() as usize) - (source.as_ptr() as usize)
}

fn respan_to<'list, 'filestr>(
    substr_span: &'list WithSpan<'filestr, ()>,
    target_substr: &'filestr str,
) -> WithSpan<'filestr, ()> {
    let offset = index_of_substr(substr_span.context, target_substr);
    WithSpan {
        data: (),
        context: substr_span.context,
        range: offset..offset + target_substr.len(),
    }
}


pub struct TrimEscapeStrList<'list, 'filestr>(
    char,
    &'static [char],
    &'static [&'static str],
    &'list [WithSpan<'filestr, ()>]
);
impl<'list, 'filestr> Print for TrimEscapeStrList<'list, 'filestr> {
    // Must perform two steps, refine the width of the array
    // and then trim the first and last elements of this refined array
    precalculate_capacity_and_build!(self, buffer {
        let Self(quote, candidates, escaped, original) = self;
        // Find the indices that contain non-whitespace entries
        let begin = original.iter()
            .position(|s| !s.as_str().trim_start().is_empty())
            .unwrap_or(if original.is_empty() { 0 } else { original.len() - 1 });
        let close = original.iter()
            .rposition(|s| !s.as_str().trim_end().is_empty())
            .unwrap_or(0);
        debug_assert!(begin <= close);

        let trimmed_list = &original[begin..cmp::min(original.len(), close + 1)];
        let trimmed_len = trimmed_list.len();
        // Number of items excluding first and last (- 2)
        let middle_len = if trimmed_len < 2 { 0 } else { trimmed_len - 2 };

        //print!("{},{} {} {:?} {:?}\n", middle_len, trimmed_len, close-begin, trimmed_list.iter().enumerate().take(middle_len).skip(1).fold("", |_, (i, a)| a.as_str()), "");

        let mut iter = trimmed_list.iter();
        let first = iter.next().map(|span| {
            let s = span.as_str();
            respan_to(span, if trimmed_len < 2 { s.trim() } else { s.trim_start() })
        });
    } {
        quote.len_utf8() => buffer.push(*quote);

        // Print 'trim_start()' or 'trim()' for the first element
        first.map(|s| EscapedStr(candidates, escaped, &s).string_len()).unwrap_or(0)
            => first.map(|s| EscapedStr(candidates, escaped, &s).push_string_into(buffer));
        // NOTE: iter.next() called once already
        iter.take(middle_len).map(|s|
            EscapedStr(candidates, escaped, s).string_len()
        ).sum::<usize>() => iter.take(middle_len).for_each(|s|
            EscapedStr(candidates, escaped, s).push_string_into(buffer)
        );

        // If trimmed_len >= 2, then we are printing a 'trim_end()'
        if begin != close {
            EscapedStr(candidates, escaped, &original[close]).string_len()
        } else {
            0
        } => if begin != close {
            EscapedStr(candidates, escaped, &original[close]).push_string_into(buffer);
        };

        quote.len_utf8() => buffer.push(*quote);
    });
}

#[macro_export]
macro_rules! define_buttons {
    (@KEYS $varname:ident $test_fn:ident { $($from:ident => $into:literal, )* }) => {
        $crate::define_buttons!(@define $crate::constants::KEYCODES
            |> $varname $test_fn { $( $from => $into, )* }
        );
    };
    (@MODS $varname:ident $test_fn:ident { $($from:ident => $into:literal, )* }) => {
        $crate::define_buttons!(@define $crate::constants::MODIFIERS
            |> $varname $test_fn { $( $from => $into, )* }
        );
    };

    (@define $FROM:path |> $INTO:ident $test_fn:ident {
        $( $from:ident => $into:literal, )*
    }) => {
        const $INTO: [&str; $FROM.len()] = {
            let mut temp = $FROM;
            $( temp[$crate::constants::Modifiers::$from as usize] = $into; )*
            temp
        };

        #[test]
        fn $test_fn() {
            use $crate::constants::Modifiers;
            // testing for duplicates
            let mut buffer1 = vec![$( Modifiers::$from as usize, )*];
            let mut buffer2 = vec![$( $into, )*];
            let len = buffer1.len();
            buffer1.sort_unstable();
            buffer1.dedup();
            buffer2.sort_unstable();
            buffer2.dedup();
            debug_assert_eq!(len, buffer1.len(),
                "Trying to map a chord modifier to two differen values");
            debug_assert_eq!(len, buffer2.len(),
                "Tryping ot map two chord modifiers to the same representation");
        }
    };
}

pub struct DeserialisedChord<'list, 'filestr>(
    &'static str,
    &'list WithSpan<'filestr, Chord>,
    &'static [&'static str],
    &'static [&'static str],
);
impl<'list, 'filestr> Print for DeserialisedChord<'list, 'filestr> {
    precalculate_capacity_and_build!(self, buffer {
        let DeserialisedChord(delim, chord_span, keycodes, modifiers) = self;
        let Chord { key, modifiers: mods } = chord_span.data;
        let mut mod_iter = modifiers.iter().enumerate()
            .filter(|(i, _)| mods & (1 << i) != 0);
        let first = mod_iter.next();
    } {
        // Process the first element separately to simulate a join()
        first.map(|(_, mod_str)| mod_str.len()).unwrap_or(0) =>
            if let Some((_, mod_str)) = first {
                buffer.push_str(mod_str);
            };
        mod_iter.map(|(_, mod_str)| mod_str.len() + delim.len()).sum::<usize>() =>
            mod_iter.for_each(|(_, mod_str)| {
                buffer.push_str(delim);
                buffer.push_str(mod_str);
            });


        // Then the key itself
        if key < keycodes.len() {
            keycodes[key].len() + if first.is_some() { delim.len() } else { 0 }
        } else {
            0
        } => if key < keycodes.len() {
             if first.is_some() {
                 buffer.push_str(delim);
             }
             buffer.push_str(keycodes[key]);
        };
    });
}

// Cannot get this to work properly so we using array!() instead

//pub fn print_array<A, T: Print>(hello: &[A], map: impl Fn(&A) -> T, buffer: &mut String) {
//    hello.iter().for_each(|x| {
//        map(x).push_string_into(buffer);
//    })
//}
//pub struct PrintArray<'list, T: Print>(&'static str, &'list [T]);
//impl<'list, T: Print> Print for PrintArray<'list, T> {
//    precalculate_capacity_and_build!(self, buffer {
//        let PrintArray(delim, list) = self;
//        let mut iter = list.iter();
//        let first = iter.next();
//    } {
//        first.map(|x| x.string_len()).unwrap_or(0) => if let Some(x) = first {
//            x.push_string_into(buffer);
//        };
//
//        iter.map(|x| delim.len() + x.string_len()).sum::<usize>() => iter.for_each(|x| {
//            buffer.push_str(delim);
//            x.push_string_into(buffer);
//        });
//    });
//}


use crate::structs::Cursor;
struct DelimSplit<'a, 'b> {
    source: &'a str,
    iter: std::str::Chars<'a>,
    delim: &'b [char],
    peek_delim: Option<&'a str>,
    //peek: Option<&'a str>,
    cursor: Cursor,
}
impl<'a, 'b> DelimSplit<'a, 'b> {
    fn new(substr: &'a str, delim: &'b [char]) -> Self {
        Self {
            source: substr,
            iter: substr.chars(),
            delim,
            peek_delim: Some(""), // Init to at least one iteration
            cursor: Cursor(0),
        }
    }
}
impl<'a, 'b> Iterator for DelimSplit<'a, 'b> {
    type Item = (&'a str, &'a str);
    fn next(&mut self) -> Option<Self::Item> {
        let Self { source, iter, delim, peek_delim, cursor } = self;
        debug_assert_eq!(None, {
            let mut a = "".chars();
            a.next();
            a.next()
        });

        //iter.position(|c| c.next());

        peek_delim.map(|prev_delim| {
            let (section, next_delim) = iter
                .scan(cursor.0, |index, c| {
                    let till_c = *index;
                    let substr = &source[cursor.span_to(till_c)];
                    *index += c.len_utf8();
                    Some((c, till_c, *index, substr))
                })
                .find(|(c, _, _, _)| delim.contains(c))
                .map(|(_, index, rindex, substr_till_c)| {
                    cursor.move_to(rindex);
                    (substr_till_c, Some(&source[index..rindex]))
                })
                .unwrap_or_else(|| (&source[cursor.move_to(source.len())], None));
            *peek_delim = next_delim;
            (prev_delim, section)
        })
    }
}

#[test]
fn custom_split() {
    let escape_list = ['"', '\\'];
    //let mut iter = "asdf".chars();
    //println!("{:?}", iter.position(|c| c == 'd'));
    //println!("{:?}", iter.next());

    let example = "a";
    let mut delim_iter = DelimSplit::new(example, &escape_list[..]);
    let mut split_iter = example.split(&escape_list[..]);
    debug_assert_eq!(Some(("", "a")), delim_iter.next());
    debug_assert_eq!(None, delim_iter.next());
    debug_assert_eq!(None, delim_iter.next());
    debug_assert_eq!(Some("a"), split_iter.next());
    debug_assert_eq!(None, split_iter.next());

    let example = "";
    let mut delim_iter = DelimSplit::new(example, &escape_list[..]);
    let mut split_iter = example.split(&escape_list[..]);
    debug_assert_eq!(Some(("", "")), delim_iter.next());
    debug_assert_eq!(None, delim_iter.next());
    debug_assert_eq!(None, delim_iter.next());
    debug_assert_eq!(Some(""), split_iter.next());
    debug_assert_eq!(None, split_iter.next());

    let example = "\\";
    let mut delim_iter = DelimSplit::new(example, &escape_list[..]);
    let mut split_iter = example.split(&escape_list[..]);
    debug_assert_eq!(Some(("", "")), delim_iter.next());
    debug_assert_eq!(Some(("\\", "")), delim_iter.next());
    debug_assert_eq!(None, delim_iter.next());
    debug_assert_eq!(None, delim_iter.next());
    debug_assert_eq!(Some(""), split_iter.next());
    debug_assert_eq!(Some(""), split_iter.next());
    debug_assert_eq!(None, split_iter.next());

    let example = "a\\b\\c";
    let mut delim_iter = DelimSplit::new(example, &escape_list[..]);
    let mut split_iter = example.split(&escape_list[..]);
    debug_assert_eq!(Some(("", "a")), delim_iter.next());
    debug_assert_eq!(Some(("\\", "b")), delim_iter.next());
    debug_assert_eq!(Some(("\\", "c")), delim_iter.next());
    debug_assert_eq!(None, delim_iter.next());
    debug_assert_eq!(None, delim_iter.next());
    debug_assert_eq!(Some("a"), split_iter.next());
    debug_assert_eq!(Some("b"), split_iter.next());
    debug_assert_eq!(Some("c"), split_iter.next());
    debug_assert_eq!(None, split_iter.next());

    let example = "\\你\"好\\嗎";
    let mut delim_iter = DelimSplit::new(example, &escape_list[..]);
    let mut split_iter = example.split(&escape_list[..]);
    debug_assert_eq!(Some(("", "")), delim_iter.next());
    debug_assert_eq!(Some(("\\", "你")), delim_iter.next());
    debug_assert_eq!(Some(("\"", "好")), delim_iter.next());
    debug_assert_eq!(Some(("\\", "嗎")), delim_iter.next());
    debug_assert_eq!(None, delim_iter.next());
    debug_assert_eq!(None, delim_iter.next());
    debug_assert_eq!(Some(""), split_iter.next());
    debug_assert_eq!(Some("你"), split_iter.next());
    debug_assert_eq!(Some("好"), split_iter.next());
    debug_assert_eq!(Some("嗎"), split_iter.next());
    debug_assert_eq!(None, split_iter.next());

    let example = "你\"h\\嗎\"";
    let mut delim_iter = DelimSplit::new(example, &escape_list[..]);
    let mut split_iter = example.split(&escape_list[..]);
    debug_assert_eq!(Some(("", "你")), delim_iter.next());
    debug_assert_eq!(Some(("\"", "h")), delim_iter.next());
    debug_assert_eq!(Some(("\\", "嗎")), delim_iter.next());
    debug_assert_eq!(Some(("\"", "")), delim_iter.next());
    debug_assert_eq!(None, delim_iter.next());
    debug_assert_eq!(None, delim_iter.next());
    debug_assert_eq!(Some("你"), split_iter.next());
    debug_assert_eq!(Some("h"), split_iter.next());
    debug_assert_eq!(Some("嗎"), split_iter.next());
    debug_assert_eq!(Some(""), split_iter.next());
    debug_assert_eq!(None, split_iter.next());
}
