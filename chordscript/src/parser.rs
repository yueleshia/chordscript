use std::fmt;
use std::ops::Range;

use crate::constants::{KEYCODES, MODIFIERS};
use crate::reporter::MarkupError;

pub mod keyspaces;
pub mod shortcuts;
pub mod lexemes;

//run: cargo test -- --nocapture

////////////////////////////////////////////////////////////////////////////////

pub fn parse_to_shortcuts(input: &str) -> Result<shortcuts::ShortcutOwner, MarkupError>  {
    let lexemes = lexemes::lex(input)?;
    shortcuts::parse_unsorted(lexemes)
}

////////////////////////////////////////////////////////////////////////////////


#[derive(Clone)]
pub struct WithSpan<'filestr, T> {
    pub data: T,
    pub context: &'filestr str,
    pub source: &'filestr str,
}

impl<'filestr, T: fmt::Debug> fmt::Debug for WithSpan<'filestr, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("WithSpan")
            .field("chord", &self.data)
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

// @TODO: make this private when we strip out sources and context
pub type ChordModifiers = u8;

#[derive(Clone, Debug)]
pub struct InnerChord {
    pub key: usize,
    pub modifiers: ChordModifiers,
}

impl InnerChord {
    pub const fn new() -> Self {
        Self {
            key: KEYCODES.len(), // Invalid index, i.e.  means None
            modifiers: 0,
        }
    }
}

#[derive(Clone)]
pub struct Chord<'filestr> {
    pub chord: InnerChord,
    pub sources: [&'filestr str; MODIFIERS.len() + 1],
    pub context: &'filestr str,
}

impl<'filestr> fmt::Debug for Chord<'filestr> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Chord")
            .field("Chord", &self.chord)
            .field("sources", &self.sources)
            .finish()
    }
}

impl<'filestr> Chord<'filestr> {
    pub fn new(context: &'filestr str) -> Self {
        Self {
            chord: InnerChord::new(),
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
        match self.chord.key.cmp(&other.chord.key) {
            std::cmp::Ordering::Equal => self.chord.modifiers.cmp(&other.chord.modifiers),
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
        self.chord.key == other.chord.key && self.chord.modifiers == other.chord.modifiers
    }
}
impl<'filestr> std::cmp::Eq for Chord<'filestr> {}

#[test]
fn chord_modifiers_big_enough() {
    use crate::constants::MODIFIERS;
    use std::mem;

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


