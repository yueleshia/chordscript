use crate::parser::shortcuts::ShortcutOwner;
//use crate::structs::{Chord, ChordModifiers};
use crate::constants::{KEYCODES, MODIFIERS};
use crate::sidebyside_len_and_push;
use crate::structs::Shortcut;

use super::{DeserialiseChord, DeserialiseHotkey, PreallocPush};

//run: cargo test -- --nocapture

//// @TODO: Extract chord into WithSpan<Chord>, would remove one lifetime
//struct DeserialisedChord {
//    delim: &'static str,
//    key: usize,
//    modifiers: ChordModifiers,
//    mod_to_str: &'static [&'static str],
//    key_to_str: &'static [&'static str],
//}

pub const SHELL_CONSTANTS: DeserialiseChord = DeserialiseChord {
    delim: " ",
    mod_to_str: &MODIFIERS,
    key_to_str: &KEYCODES,
};
pub const SHELL_CHORD_DELIM: &str = " ; ";

impl<'owner, 'filestr> PreallocPush<'filestr, DeserialiseChord> for Shortcut<'owner, 'filestr> {
    sidebyside_len_and_push!(len, push_into, self: &Self, extra: DeserialiseChord, buffer: 'filestr {} {
        " '";
        DeserialiseHotkey(SHELL_CHORD_DELIM, self.hotkey).len(extra) => DeserialiseHotkey(" ; ", self.hotkey).push_into(extra, buffer);
        "')  ";
        self.command.len() => for with_span in self.command {
            buffer.push(with_span.source);
        };
        "\n";
    });
}

pub struct Wrapper();
impl<'a, 'b> PreallocPush<'a, &'b ShortcutOwner<'a>> for Wrapper {
    sidebyside_len_and_push!(len, push_into, self: &Self, owner: &ShortcutOwner<'a>, buffer: 'a {} {
        "#!/bin/sh\n";
        "case \"${1}\"\n";
        owner.to_iter().map(|s| 1 + s.len(SHELL_CONSTANTS)).sum::<usize>() => {
            let mut prefix = "in";
            owner.to_iter().filter(|s| !s.is_placeholder).for_each(|s| {
                buffer.push(prefix);
                prefix = ";;";
                s.push_into(SHELL_CONSTANTS, buffer);
            });
        };
        "*)  notify.sh \"invalid key combination ${1}\"; exit 1\n";
        "esac\n";
    });
}
