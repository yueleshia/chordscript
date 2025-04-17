//run: cargo test -- --nocapture

use crate::constants::{KEYCODES, MODIFIERS};
//use crate::reporter::MarkupError;
use std::ops::Range;

#[derive(Clone, Debug)]
pub struct WithSpan<'filestr, T> {
    pub data: T,
    pub context: &'filestr str,
    pub source: &'filestr str,
}

impl<'filestr, T> WithSpan<'filestr, T> {
    //pub fn to_error(&self, message: &str) -> MarkupError {
    //    MarkupError::from_str(&self.context, self.as_str(), message.to_string())
    //}

    pub fn map_to<U>(&self, new_value: U) -> WithSpan<'filestr, U> {
        WithSpan {
            data: new_value,
            context: self.context,
            source: self.source,
        }
    }

    //pub fn span_to_as_range(&self, to: &Self) -> Range<usize> {
    //    // This an internal structure so debug_assert is fine
    //    debug_assert_eq!(self.context, to.context);
    //    self.range.start..to.range.end
    //}
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
    pub fn span_to(&self, index: usize) -> Range<usize> {
        self.0..index
    }
    pub fn width(&self, index: usize) -> usize {
        debug_assert!(index >= self.0);
        index - self.0
    }
}

type ChordModifiers = u8;

#[derive(Clone, Debug)]
pub struct Chord<'filestr> {
    pub key: usize,
    pub modifiers: ChordModifiers,
    pub sources: [&'filestr str; MODIFIERS.len() + 1],
    pub context: &'filestr str,
}

impl<'filestr> Chord<'filestr> {
    pub fn new(context: &'filestr str) -> Self {
        Self {
            key: KEYCODES.len(), // Invalid index, i.e.  means None
            modifiers: 0,
            sources: [&context[0..0]; MODIFIERS.len() + 1],
            context,
        }
    }
}

impl<'filestr> std::cmp::Ord for Chord<'filestr> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.key.cmp(&other.key) {
            std::cmp::Ordering::Equal => self.modifiers.cmp(&other.modifiers),
            a => a,
        }
    }
}
impl<'filestr> std::cmp::PartialOrd for Chord<'filestr> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<'filestr> std::cmp::PartialEq for Chord<'filestr> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.modifiers == other.modifiers
    }
}
impl<'filestr> std::cmp::Eq for Chord<'filestr> {}

#[test]
fn chord_modifiers_big_enough() {
    use std::mem;
    use crate::constants::MODIFIERS;

    let modifier_size = mem::size_of::<ChordModifiers>() * 8;
    assert!(
        modifier_size >= MODIFIERS.len(),
        "'Modifiers' is not large enough to hold all the flags"
    );
}

pub type Hotkey<'owner, 'filestr> = &'owner [Chord<'filestr>];

#[derive(Clone, Debug)]
pub struct Shortcut<'owner, 'filestr> {
    pub is_placeholder: bool,
    pub hotkey: Hotkey<'owner, 'filestr>,
    pub command: &'owner [WithSpan<'filestr, ()>],
}
