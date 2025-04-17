//run: cargo test -- --nocapture
// run: cargo run -- shortcuts -c $HOME/interim/hk/config.txt

// Following the theme of this entire project, we calculating the exact
// memory required for printing to save on allocations instead of implementing
// '.to_string()' (i.e. impl std::fmt::Display).
//
// This is re-exports the print adaptors for various hotkey formats
// This also has the Print trait and various macros/functions that are used
// for the modules used inside of here
//
// Technically reporter.rs probably should also exist inside this

macro_rules! reexport {
    ($mod:ident ::*) => {
        mod $mod;
        pub use $mod::*;
    };
}

reexport!(keyspace_preview::*); // Default printer
reexport!(list_preview::*); // Default printer
reexport!(shellscript::*); // For external file to be used by others
//
//reexport!(i3::*); // No way to escape newlines, so should avoid this
reexport!(i3_shell::*);
// @TODO leftwm
// @TODO sxhkd
// @TODO dwm
// @TODO external shellscript that handles adapting for us

use std::cmp;

use crate::constants::{KEYCODES,MODIFIERS};
use crate::precalculate_capacity_and_build;
use crate::structs::{InnerChord, Chord, Cursor, WithSpan};


/****************************************************************************
 * Print trait
 ****************************************************************************/
// This works by building up a buffer instead of allocating a buffer on
// every 'to_string()' call
pub trait Print {
    fn string_len(&self) -> usize;
    fn push_string_into(&self, buffer: &mut String);

    // @TODO impl a direct print to stdout buffer and benchmark

    #[cfg(debug_assertions)]
    fn to_string_custom(&self) -> String;

    fn print_stdout(&self) {
        let mut buffer = String::with_capacity(self.string_len());
        self.push_string_into(&mut buffer);
        print!("{}", buffer);
    }

    fn print_stderr(&self) {
        let mut buffer = String::with_capacity(self.string_len());
        self.push_string_into(&mut buffer);
        eprint!("{}", buffer);
    }
}


pub trait PrintError<T> {
    fn or_die(self, exit_code: i32) -> T;
}

impl<T, E: Print> PrintError<T> for Result<T, E> {
    fn or_die(self, exit_code: i32) -> T {
        match self {
            Ok(x) => x,
            Err(err) => {
                let mut msg = String::with_capacity(err.string_len());
                err.push_string_into(&mut msg);

                eprintln!("{}", msg);
                std::process::exit(exit_code)
            }
        }
    }
}

/****************************************************************************
 * Macros
 ****************************************************************************/
// For dealing with an array views of printable elements
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

// This essentially copies constants::KEYCODES or constants::MODIFIERS, and
// allows you to do a non-exhaustive match, to set each value
//
// constants::KEYCODES and constants::MODIFIERS are for parsing the input file
// This is for setting the output substr for all the keys for use inside of
// ListChord
// The default print behaviour (ListPreview and KeyspacePreview) does not remap
#[macro_export]
macro_rules! define_buttons {
    (@KEYS $varname:ident $test_fn:ident { $($from:ident => $into:literal, )* }) => {
        $crate::define_buttons!(@define
            $crate::constants::KEYCODES, $crate::constants::Keycodes, Keycodes
            |> $varname $test_fn { $( $from => $into, )* }
        );
    };
    (@MODS $varname:ident $test_fn:ident { $($from:ident => $into:literal, )* }) => {
        $crate::define_buttons!(@define
            $crate::constants::MODIFIERS, $crate::constants::Modifiers, Modifiers
            |> $varname $test_fn { $( $from => $into, )* }
        );
    };

    (@define $FROM_STRS:path, $from_enum_path:path, $FromEnum:ident
        |> $INTO:ident $test_fn:ident {
        $( $from:ident => $into:literal, )*
    }) => {
        const $INTO: [&str; $FROM_STRS.len()] = {
            let mut temp = $FROM_STRS;
            use $from_enum_path;
            $( temp[$FromEnum::$from as usize] = $into; )*
            temp
        };

        #[test]
        fn $test_fn() {
            use $from_enum_path;

            // Check that the input is ordered in the same way for clarity
            {
                let pre_sorted = vec![$( $FromEnum::$from as usize, )*];
                let mut postsorted = pre_sorted.clone();
                postsorted.sort_unstable();
                assert_eq!(pre_sorted, postsorted, "{} {}",
                    "Enter in the same order specified in constants.rs.",
                    "This helps for consistency.",
                );
            }

            // testing for duplicates
            {
                let mut sorted_buffer1 = vec![$( $FromEnum::$from as usize, )*];
                let mut sorted_buffer2 = vec![$( $into, )*];
                let len = sorted_buffer1.len();
                sorted_buffer1.sort_unstable();
                sorted_buffer1.dedup();
                sorted_buffer2.sort_unstable();
                sorted_buffer2.dedup();
                assert_eq!(len, sorted_buffer1.len(),
                    "Trying to map a button to two different values");
                assert_eq!(len, sorted_buffer2.len(),
                    "Trying to map two button to the same representation");
            }

        }
    };
}

/****************************************************************************
 * Print implementations for essentially '&[&str]', '&str', and 'Chord'
 ****************************************************************************/
pub struct EscapedStrList<'list, 'filestr>(
    char,
    &'static [char],
    &'static [&'static str],
    &'list [WithSpan<'filestr, ()>],
);
struct EscapedStr<'list, 'filestr>(
    &'static [char],
    &'static [&'static str],
    &'list WithSpan<'filestr, ()>,
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

        let mut iter = DelimSplit::new(substr_span.source, candidates);
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
        new_iter.map(|(escaped, s)| escaped.len() + s.len()).sum::<usize>() =>
            new_iter.for_each(|(escaped_delim, s)| {
                buffer.push_str(escaped_delim);
                buffer.push_str(s);
            });
    });
}

pub struct TrimEscapeStrList<'list, 'filestr>(
    char,
    &'static [char],
    &'static [&'static str],
    &'list [WithSpan<'filestr, ()>],
);
impl<'list, 'filestr> Print for TrimEscapeStrList<'list, 'filestr> {
    // Must perform two steps, refine the width of the array
    // and then trim the first and last elements of this refined array
    precalculate_capacity_and_build!(self, buffer {
        let Self(quote, candidates, escape, original) = self;
        // Find the indices that contain non-whitespace entries
        let begin = original.iter()
            .position(|s| !s.source.trim_start().is_empty())
            .unwrap_or(if original.is_empty() { 0 } else { original.len() - 1 });
        let close = original.iter()
            .rposition(|s| !s.source.trim_end().is_empty())
            .unwrap_or(0);
        debug_assert!(begin <= close);

        let trimmed_list = &original[begin..cmp::min(original.len(), close + 1)];
        let trimmed_len = trimmed_list.len();
        // Number of items excluding first and last (- 2)
        let middle_len = if trimmed_len < 2 { 0 } else { trimmed_len - 2 };

        //print!("{},{} {:?} {:?}\n",
        //    middle_len,
        //    trimmed_len,
        //    original[close].as_str(),
        //    trimmed_list.iter().enumerate()
        //        .take(middle_len)
        //        .skip(1)
        //        .fold("", |_, (_, a)| a.as_str()),
        //);

        let mut iter = trimmed_list.iter();

        // Have to own the span here
        let first = iter.next().map(|span| {
            let s = span.source;
            WithSpan {
                data: (),
                context: span.context,
                source: if trimmed_len < 2 { s.trim() } else { s.trim_start() },
            }
            //respan_to(span, if trimmed_len < 2 { s.trim() } else { s.trim_start() })
        });
        let finis = if begin != close {
            Some (WithSpan {
                data: (),
                context: original[close].context,
                source: original[close].source.trim_end(),
            })
            //Some(respan_to(&original[close], original[close].as_str().trim_end()))
        } else {
            None
        };
    } {
        quote.len_utf8() => buffer.push(*quote);

        // Print 'trim_start()' or 'trim()' for the first element
        first.map(|s| EscapedStr(candidates, escape, &s).string_len()).unwrap_or(0)
            => first.map(|s| EscapedStr(candidates, escape, &s).push_string_into(buffer));

        // NOTE: iter.next() called once already (skipping first)
        iter.take(middle_len).map(|s|
            EscapedStr(candidates, escape, s).string_len()
        ).sum::<usize>() => iter.take(middle_len).for_each(|s|
            EscapedStr(candidates, escape, s).push_string_into(buffer)
        );

        // If trimmed_len >= 2, then we are printing a 'trim_end()'
        finis.map(|s| EscapedStr(candidates, escape, &s).string_len()).unwrap_or(0) =>
            finis.map(|s| EscapedStr(candidates, escape, &s).push_string_into(buffer));

        //if begin != close {
        //    EscapedStr(candidates, escape, &original[close]).string_len()
        //} else {
        //    0
        //} => if begin != close {
        //    EscapedStr(candidates, escape, &original[close]).push_string_into(buffer);
        //};

        quote.len_utf8() => buffer.push(*quote);
    });
}

pub struct DeserialisedChord<'list, 'filestr>(
    &'static str,
    &'list Chord<'filestr>,
    &'static [&'static str],
    &'static [&'static str],
);
impl<'list, 'filestr> Print for DeserialisedChord<'list, 'filestr> {
    precalculate_capacity_and_build!(self, buffer {
        let DeserialisedChord(delim, chord, keycodes, modifiers) = self;
        let InnerChord { key, modifiers: mods } = chord.chord;
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

#[inline]
fn default_print_chord<'a, 'b>(chord: &'a Chord<'b>) -> DeserialisedChord<'a, 'b> {
    DeserialisedChord(" ", chord, &KEYCODES, &MODIFIERS)
}

/****************************************************************************
 * Helper functions
 ****************************************************************************/
//pub fn index_of_substr<'a>(source: &'a str, substr: &'a str) -> usize {
//    (substr.as_ptr() as usize) - (source.as_ptr() as usize)
//}

//fn respan_to<'list, 'filestr>(
//    substr_span: &'list WithSpan<'filestr, ()>,
//    target_substr: &'filestr str,
//) -> WithSpan<'filestr, ()> {
//    let offset = index_of_substr(substr_span.context, target_substr);
//    WithSpan {
//        data: (),
//        context: substr_span.context,
//        range: offset..offset + target_substr.len(),
//    }
//}

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
        let Self {
            source,
            iter,
            delim,
            peek_delim,
            cursor,
        } = self;
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
