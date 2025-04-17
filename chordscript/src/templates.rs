//run: cargo test -- --nocapture

use crate::constants::{KEYCODES, MODIFIERS};
use crate::parser::shortcuts::ShortcutOwner;
use crate::parser::{Chord, InnerChord};
use std::io::{Result as IoResult, Write};

use crate::{sidebyside_len_and_push, array_index_by_enum};

mod shellscript;
mod i3_shell;
mod debug_shortcuts;

array_index_by_enum! { TEMPLATE_COUNT: usize
    pub enum Templates {
        ShellScript    => Templates::ShellScript    => "shell"           => &shellscript::Wrapper(),
        I3Shell        => Templates::I3Shell        => "i3"              => &i3_shell::Wrapper(),
        DebugShortcuts => Templates::DebugShortcuts => "debug-shortcuts" => &debug_shortcuts::Wrapper(),
    }
    => 1 pub const ID_TO_TEMPLATE: [Templates]
    => 2 pub const ID_TO_STR: [&str]
    => 3 pub const VTABLE: [&dyn for<'a, 'b> PreallocPush<'a, &'b ShortcutOwner<'a>>]
}
//array_index_by_enum! {
//    pub enum Templates
//    => pub const ID_TO_TEMPLATE: [Templates]
//    => pub const ID_TO_STR: [&str]
//    => const VTABLE: [&dyn for<'a, 'b> PreallocPush<'a, &'b ShortcutOwner<'a>>]
//    = {
//        ShellScript => "shell" => &shellscript::Wrapper(),
//        I3Shell =>  "i3" => &i3_shell::Wrapper(),
//        DebugShortcuts =>  "debug-shortcuts" => &debug_shortcuts::Wrapper(),
//    };
//}

//#[test]
//fn asdf() {
//    use crate::parser;
//    use std::io::stdout;
//    let me = include_str!(concat!(env!("HOME"), "/.config/rc/wm-shortcuts"));
//    //print("|{}|", me);
//    let _lock = &mut stdout();
//
//    let ast = &parser::parse_to_shortcuts(me).unwrap();
//    Templates::ShellScript
//        .pipe(ast, _lock)
//        .expect("unreachable");
//}

impl<'filestr, 'b> PreallocPush<'filestr, &'b ShortcutOwner<'filestr>> for Templates {
    fn len(&self, owner: &'b ShortcutOwner<'filestr>) -> usize {
        VTABLE[self.id()].len(owner)
    }
    fn push_into(&self, owner: &'b ShortcutOwner<'filestr>, buffer: &mut Vec<&'filestr str>) {
        VTABLE[self.id()].push_into(owner, buffer);
    }
}

impl Templates {
    #[allow(dead_code)]
    pub fn pipe<T: Write>(&self, shortcuts: &ShortcutOwner, stream: &mut T) -> IoResult<()> {
        let wrapper = VTABLE[self.id()];
        let mut buffer = Vec::with_capacity(wrapper.len(shortcuts));
        wrapper.push_into(shortcuts, &mut buffer);

        for x in buffer {
            stream.write_fmt(format_args!("{}", x))?
        }
        Ok(())
    }
}

const DEBUG_CHORD_CONSTANTS: DeserialiseChord = DeserialiseChord {
    delim: " ; ",
    mod_to_str: &MODIFIERS,
    key_to_str: &KEYCODES,
};
const CHORD_MAX_PUSH_LEN: usize = InnerChord::new().len(DEBUG_CHORD_CONSTANTS);

pub trait PreallocPush<'a, T> {
    fn len(&self, extra: T) -> usize;
    fn push_into(&self, extra: T, buffer: &mut Vec<&'a str>);
}

// @TODO: Is it a bug that this is pub? For `i3shell.rs` to be able to use {shellscript::SHELL_CONSTANTS}
#[derive(Clone, Copy)]
pub struct DeserialiseChord {
    delim: &'static str,
    mod_to_str: &'static [&'static str],
    key_to_str: &'static [&'static str],
}

impl InnerChord {
    sidebyside_len_and_push!(! const ! len, push_into<'a> (self: &Self, extra: DeserialiseChord, buffer: 'a) {} {
        // At most `mod_to_str.len()` modifiers will be added
        extra.mod_to_str.len() * 2 => {
            let mut delim = "";
            for (i, mod_str) in extra.mod_to_str.iter().enumerate() {
                if self.modifiers & (1 << i) != 0 {
                    buffer.push(delim);
                    buffer.push(mod_str);
                    delim = extra.delim;
                }
            }
        };

        // Zero or two &str added
        2 => if self.key < extra.key_to_str.len() {
            buffer.push(extra.delim);
            buffer.push(extra.key_to_str[self.key]);
        };
    });
}

//impl<'a> PreallocPush<'a, DeserialiseChord> for Chord<'a> {
//    fn len(&self, extra: DeserialiseChord) -> usize { chord_len(self, extra) }
//    fn push_into(&self, extra: DeserialiseChord, buffer: &mut Vec<&'a str>) { chord_push_into(self, extra, buffer) }
//}

struct DeserialiseHotkey<'a, 'b>(&'static str, &'b [Chord<'a>]);
impl<'a, 'b> PreallocPush<'a, DeserialiseChord> for DeserialiseHotkey<'a, 'b> {
    fn len(&self, _: DeserialiseChord) -> usize {
        self.1.len() * (1 + CHORD_MAX_PUSH_LEN)
    }
    fn push_into(&self, extra: DeserialiseChord, buffer: &mut Vec<&'a str>) {
        let mut delim = "";
        for sourced_chord in self.1 {
            buffer.push(delim);
            delim = self.0;
            sourced_chord.chord.push_into(extra, buffer);
        }
    }
}

//impl<'a, 'b, T: PreallocPush<'a, U>, U: Copy> PreallocPush<'a, U> for List<'b, T, U> {
//    fn len(&self, extra: U) -> usize {
//        self.1.len() * 1 + self.1.iter().map(|x| x.len(extra)).sum::<usize>()
//    }
//    fn push_into(&self, extra: U, buffer: &mut Vec<&str>) {
//        let mut joiner = "";
//        for x in self.1 {
//            buffer.push(joiner);
//            joiner = self.0;
//            x.push_into(extra, buffer);
//        }
//    }
//}
