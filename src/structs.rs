//run: cargo test -- --nocapture

use crate::constants::{KEYCODES, MODIFIERS};
use crate::reporter::MarkupError;
use std::fmt;
use std::ops::Range;

#[derive(Clone, Debug)]
pub struct WithSpan<'filestr, T>(pub T, pub &'filestr str, pub Range<usize>);

impl<'filestr, T> WithSpan<'filestr, T> {
    pub fn to_error(&self, message: &str) -> MarkupError {
        MarkupError::from_str(&self.1, self.as_str(), message.to_string())
    }

    pub fn as_str(&self) -> &'filestr str {
        &self.1[self.2.start..self.2.end]
    }

    pub fn source(&self) -> &str {
        self.1
    }

    pub fn span_to_as_range(&self, to: &Self) -> (usize, usize) {
        // This an internal structure so debug_assert is fine
        debug_assert_eq!(self.1, to.1);
        (self.2.start, to.2.end)
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

pub type Hotkey<'owner> = &'owner [Chord];
pub fn debug_hotkey_to_string(hotkey: Hotkey) -> String {
    hotkey
        .iter()
        .map(|chord| chord.to_string())
        .collect::<Vec<_>>()
        .join(" ; ")
}

#[derive(Debug)]
pub struct Shortcut<'owner, 'filestr> {
    pub hotkey: Hotkey<'owner>,
    pub command: &'owner [&'filestr str],
}
impl<'owner, 'filestr> fmt::Display for Shortcut<'owner, 'filestr> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("|")?;
        f.write_str(
            self.hotkey
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(" ; ")
                .as_str(),
        )?;
        f.write_str("|")?;
        f.write_str(self.command.join("").trim())
    }
}

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
