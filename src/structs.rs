//run: cargo test -- --nocapture

use crate::constants::{KEYCODES, MODIFIERS};
use crate::reporter::MarkupError;
use std::fmt;
use std::ops::Range;

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
pub fn debug_hotkey_to_string(hotkey: Hotkey) -> String {
    hotkey
        .iter()
        .map(|chord_span| chord_span.data.to_string())
        .collect::<Vec<_>>()
        .join(" ; ")
}

#[derive(Debug)]
pub struct Shortcut<'owner, 'filestr> {
    pub hotkey: Hotkey<'owner, 'filestr>,
    pub command: &'owner [WithSpan<'filestr, ()>],
}
impl<'owner, 'filestr> fmt::Display for Shortcut<'owner, 'filestr> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("|")?;
        f.write_str(
            self.hotkey
                .iter()
                .map(|x| x.data.to_string())
                .collect::<Vec<_>>()
                .join(" ; ")
                .as_str(),
        )?;
        f.write_str("|")?;
        f.write_str(self.command.iter().map(|span| span.as_str()).collect::<Vec<_>>().join("").trim())
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

type ChordModifiers = u8;

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct Chord {
    pub key: usize,
    pub modifiers: ChordModifiers,
}

impl Chord {
    pub fn new() -> Self {
        Self {
            key: KEYCODES.len(), // Invalid index, i.e.  means None
            modifiers: 0,
        }
    }

    pub fn copy(&mut self, chord: &Self) {
        self.key = chord.key;
        self.modifiers = chord.modifiers;
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, modifier) in MODIFIERS.iter().enumerate() {
            let flag = 1 << i;
            if self.modifiers & flag != 0 {
                f.write_str(modifier)?;
                f.write_str(" ")?;
            }
        }
        if self.key < KEYCODES.len() {
            f.write_str(KEYCODES[self.key])?;
        }
        Ok(())
    }
}

#[test]
fn chord_modifiers_big_enough() {
    use std::mem;

    let modifier_size = mem::size_of::<ChordModifiers>() * 8;
    assert!(
        modifier_size >= MODIFIERS.len(),
        "'Modifiers' is not large enough to hold all the flags"
    );
}
