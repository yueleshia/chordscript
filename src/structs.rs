//run: cargo test -- --nocapture

use crate::constants::{KEYCODES, MODIFIERS};
use std::ops::Range;
use std::fmt;

#[derive(Clone)]
pub struct WithSpan<'filestr, T> {
    pub data: T,
    pub context: &'filestr str,
    pub source: &'filestr str,
}

impl<'filestr, T: fmt::Debug> fmt::Debug for WithSpan<'filestr, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("WithSpan")
            .field("data", &self.data)
            .field("source", &self.source)
            .finish()
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
}

type ChordModifiers = u8;

#[derive(Clone)]
pub struct Chord<'filestr> {
    pub key: usize,
    pub modifiers: ChordModifiers,
    pub sources: [&'filestr str; MODIFIERS.len() + 1],
    pub context: &'filestr str,
}

impl<'filestr> fmt::Debug for Chord<'filestr> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Chord")
            .field("key", &self.key)
            .field("modifiers", &self.modifiers)
            .field("sources", &self.sources)
            .finish()
    }
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

//#[cfg(debug_assertions)]
//impl<'filestr> std::fmt::Display for Chord<'filestr> {
//    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//        write!(f, "{} {}: {:?}", self.modifiers, self.key, self.sources)
//    }
//}

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
