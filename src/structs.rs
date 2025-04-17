//run: cargo test -- --nocapture

use crate::constants::{KEYCODES, MODIFIERS};
use crate::precalculate_capacity_and_build;
use crate::reporter::MarkupError;
use std::ops::Range;

pub trait Print {
    fn string_len(&self) -> usize;
    fn push_string_into(&self, buffer: &mut String);
    // @TODO cfg(debug) only
    fn to_string_custom(&self) -> String;
}

#[derive(Clone, Debug)]
pub struct WithSpan<'filestr, T> {
    pub data: T,
    pub context: &'filestr str,
    pub range: Range<usize>,
}

impl<'filestr, T> WithSpan<'filestr, T> {
    pub fn to_error(&self, message: &str) -> MarkupError {
        MarkupError::from_str(&self.context, self.as_str(), message.to_string())
    }

    pub fn as_str(&self) -> &'filestr str {
        &self.context[self.range.start..self.range.end]
    }

    pub fn map_to<U>(&self, new_value: U) -> WithSpan<'filestr, U> {
        WithSpan {
            data: new_value,
            context: self.context,
            range: self.range.clone(),
        }
    }

    pub fn span_to_as_range(&self, to: &Self) -> (usize, usize) {
        // This an internal structure so debug_assert is fine
        debug_assert_eq!(self.context, to.context);
        (self.range.start, to.range.end)
    }
}

#[derive(Debug)]
pub struct Cursor(pub usize);

impl Cursor {
    pub fn move_to(&mut self, index: usize) -> Range<usize> {
        debug_assert!(index >= self.0);
        let from = self.0;
        self.0 = index;
        from..index
    }
    pub fn width(&self, index: usize) -> usize {
        debug_assert!(index >= self.0);
        index - self.0
    }
}

pub type Hotkey<'owner, 'filestr> = &'owner [WithSpan<'filestr, Chord>];

impl<'owner, 'filestr> Print for Hotkey<'owner, 'filestr> {
    precalculate_capacity_and_build!(self, buffer {
        let mut iter = self.iter();
        let delim = " ; ";
    } {
        if let Some(first) = iter.next() {
            first.string_len() + iter
                .map(|chord_span| delim.len() + chord_span.string_len())
                .sum::<usize>()
        } else {
            0
        } => if let Some(first) = iter.next() {
            first.push_string_into(buffer);
            iter.for_each(|chord_span| {
                buffer.push_str(delim);
                chord_span.push_string_into(buffer);

            });
        };
    });
}

type ChordModifiers = u8;

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct Chord {
    pub key: usize,
    pub modifiers: ChordModifiers,
}

impl<'owner, 'filestr> Print for WithSpan<'filestr, Chord> {
    precalculate_capacity_and_build!(self, buffer {
        let Chord { key, modifiers } = self.data;
        let mut mod_iter = MODIFIERS.iter().enumerate()
            .filter(|(i, _)| modifiers & (1 << i) != 0);
        let space = ' ';
        debug_assert!(space.len_utf8() == 1);
        let first = mod_iter.next();
    } {
        // Process the first element separately to simulate a join()
        first.map(|(_, mod_str)| mod_str.len()).unwrap_or(0) =>
            if let Some((_, mod_str)) = first {
                buffer.push_str(mod_str);
            };
        mod_iter.map(|(_, mod_str)| mod_str.len() + 1).sum::<usize>() =>
            mod_iter.for_each(|(_, mod_str)| {
                buffer.push(space);
                buffer.push_str(mod_str);
            });


        // Then the key itself
        if key < KEYCODES.len() {
            KEYCODES[key].len() + if first.is_some() { 1 } else { 0 }
        } else {
            0
        } => if key < KEYCODES.len() {
             if first.is_some() {
                 buffer.push(space);
             }
             buffer.push_str(KEYCODES[key]);
         };
    });
}

impl Chord {
    pub fn new() -> Self {
        Self {
            key: KEYCODES.len(), // Invalid index, i.e.  means None
            modifiers: 0,
        }
    }
}

impl<'owner, 'filestr> std::cmp::Ord for WithSpan<'filestr, Chord> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.data.cmp(&other.data)
    }
}
impl<'owner, 'filestr> std::cmp::PartialOrd for WithSpan<'filestr, Chord> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.data.cmp(&other.data))
    }
}
impl<'owner, 'filestr> std::cmp::PartialEq for WithSpan<'filestr, Chord> {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}
impl<'owner, 'filestr> std::cmp::Eq for WithSpan<'filestr, Chord> {}

#[test]
fn chord_modifiers_big_enough() {
    use std::mem;

    let modifier_size = mem::size_of::<ChordModifiers>() * 8;
    assert!(
        modifier_size >= MODIFIERS.len(),
        "'Modifiers' is not large enough to hold all the flags"
    );
}

#[derive(Debug)]
pub struct Shortcut<'owner, 'filestr> {
    pub hotkey: Hotkey<'owner, 'filestr>,
    pub command: &'owner [WithSpan<'filestr, ()>],
}

impl<'owner, 'filestr> Print for Shortcut<'owner, 'filestr> {
    precalculate_capacity_and_build!(self, buffer {} {
        // First the modifiers
        self.hotkey.string_len() + 2  => {
            buffer.push('|');
            self.hotkey.push_string_into(buffer);
            buffer.push('|');
        };
        self.command.iter().map(|s| s.range.len()).sum::<usize>() => {
            self.command.iter().for_each(|str_span| {
                buffer.push_str(str_span.as_str());
            });
        };

    });
}
