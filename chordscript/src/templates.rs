//run: cargo test -- --nocapture

use std::io;

use crate::constants::{KEY_UTF8_MAX_LEN, MOD_UTF8_MAX_LEN, KEYCODES, MODIFIERS};
use crate::parser::{shortcuts::ShortcutOwner, Chord, InnerChord};
use crate::{array_index_by_enum, sidebyside_len_and_push};

mod debug_shortcuts;
mod i3_shell;
mod shellscript;

//macro_rules! row {
//    ($Enum:ident :: $Variant:ident => $id:literal) => {
//        $Enum::$Variant => $id
//    };
//}
//
//
pub enum F { // Format
    N(&'static str), // Native
    S(&'static str), // Shell
}

array_index_by_enum!( TEMPLATE_COUNT: usize
    pub enum Templates {
        ShellScript    => F::N("shell")           => &shellscript::Wrapper()     => &shellscript::Wrapper(),
        I3Shell        => F::S("i3")              => &i3_shell::Wrapper()        => &i3_shell::Wrapper(),
        DebugShortcuts => F::N("debug-shortcuts") => &debug_shortcuts::Wrapper() => &debug_shortcuts::Wrapper(),
    }
    => 1 pub const ID_TO_TYPE: [F]
    => 2 pub const VTABLE_STRING: [&dyn for<'a, 'b> PreallocPush<&'b ShortcutOwner<'a>, String>]
    => 3 pub const VTABLE_STDOUT: [&dyn for<'a, 'b> PreallocPush<&'b ShortcutOwner<'a>, io::Stdout>]
);

pub trait Consumer {
    fn consume(&mut self, part: &str);
}
impl Consumer for String {
    fn consume(&mut self, part: &str) {
        // Assume strings were allocated with `with_capacity(PreallocLen)`
        #[cfg(debug_assertions)]
        let before = self.capacity();

        self.push_str(part);

        #[cfg(debug_assertions)]
        assert_eq!(before, self.capacity());
    }
}
impl Consumer for io::Stdout {
    fn consume(&mut self, part: &str) {
        use io::Write;
        match self.write_all(part.as_bytes()) {
            Ok(()) => {}
            Err(_) => unreachable!(),
        }
    }
}
impl Consumer for std::fmt::Formatter<'_> {
    fn consume(&mut self, part: &str) {
        self.write_str(part).unwrap()
    }
}

pub trait PreallocLen<T> {
    fn len(&self, extra: T) -> usize;
}
// Split this because `.len()` might be the same for various Consumers
pub trait PreallocPush<T, U: Consumer>: PreallocLen<T> {
    fn pipe(&self, extra: T, pipe: &mut U);
}

//impl Templates {
//    pub fn len(&self, owner: &ShortcutOwner<'_>) -> usize {
//        VTABLE_STDOUT[self.id()].len(owner)
//    }
//    pub fn pipe_stdout(&self, owner: &ShortcutOwner<'_>, buffer: &mut io::Stdout) {
//        VTABLE_STDOUT[self.id()].pipe(owner, buffer);
//    }
//    pub fn pipe_string(&self, owner: &ShortcutOwner<'_>, buffer: &mut String) {
//        VTABLE_STRING[self.id()].pipe(owner, buffer);
//    }
//}

const DEBUG_CHORD_CONSTANTS: DeserialiseChord = DeserialiseChord {
    delim: " ; ",
    mod_to_str: &MODIFIERS,
    key_to_str: &KEYCODES,
};
const CHORD_MAX_PUSH_LEN: usize = InnerChord::new().len(DEBUG_CHORD_CONSTANTS);

#[derive(Clone, Copy)]
pub(crate) struct DeserialiseChord {
    delim: &'static str,
    mod_to_str: &'static [&'static str],
    key_to_str: &'static [&'static str],
}

impl InnerChord {
    sidebyside_len_and_push!(! const ! len, pipe<F> (self: &Self, extra: DeserialiseChord, buffer: F) {} {
        // At most `mod_to_str.len()` modifiers will be added
        extra.mod_to_str.len() * (extra.delim.len() + MOD_UTF8_MAX_LEN)
        => {
            let mut delim = "";
            for (i, mod_str) in extra.mod_to_str.iter().enumerate() {
                if self.modifiers & (1 << i) != 0 {
                    buffer.consume(delim);
                    buffer.consume(mod_str);
                    delim = extra.delim;
                }
            }
        };

        // Zero or two &str added
        extra.delim.len() + KEY_UTF8_MAX_LEN => if self.key < extra.key_to_str.len() {
            buffer.consume(extra.delim);
            buffer.consume(extra.key_to_str[self.key]);
        };
    });
}

//impl<'a> PreallocPush<'a, DeserialiseChord> for Chord<'a> {
//    fn len(&self, extra: DeserialiseChord) -> usize { chord_len(self, extra) }
//    fn push_into(&self, extra: DeserialiseChord, buffer: &mut Vec<&'a str>) { chord_push_into(self, extra, buffer) }
//}

struct DeserialiseHotkey<'a, 'b>(&'static str, &'b [Chord<'a>]);
impl PreallocLen<DeserialiseChord> for DeserialiseHotkey<'_, '_> {
    fn len(&self, extra: DeserialiseChord) -> usize {
        self.1.len() * (extra.delim.len() + CHORD_MAX_PUSH_LEN)
    }
}
impl<U: Consumer> PreallocPush<DeserialiseChord, U> for DeserialiseHotkey<'_, '_> {
    fn pipe(&self, extra: DeserialiseChord, buffer: &mut U) {
        let mut delim = "";
        for sourced_chord in self.1 {
            buffer.consume(delim);
            buffer.consume(delim);
            delim = self.0;
            sourced_chord.chord.pipe(extra, buffer);
        }
    }
}
