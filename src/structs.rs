//run: cargo test -- --nocapture

use crate::constants::KEYCODES;
use crate::reporter::MarkupError;
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

    pub fn span_to_as_range(&self, to: &Self) -> Range<usize> {
        // This an internal structure so debug_assert is fine
        debug_assert_eq!(self.context, to.context);
        self.range.start..to.range.end
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
    pub fn span_to(&self, index: usize) -> Range<usize> {
        self.0..index
    }
    pub fn width(&self, index: usize) -> usize {
        debug_assert!(index >= self.0);
        index - self.0
    }
}

pub type Hotkey<'owner, 'filestr> = &'owner [WithSpan<'filestr, Chord>];

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
    use crate::constants::MODIFIERS;

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
