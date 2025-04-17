use crate::constants::{KEYCODES, MODIFIERS};
use crate::parser::{shortcuts::ShortcutOwner, Shortcut};
use crate::sidebyside_len_and_push;

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
sidebyside_len_and_push!(len<'b, 'filestr>, push_into<'b, 'filestr>
(shortcut: Shortcut<'b, 'filestr>, extra: DeserialiseChord, buffer: 'filestr) {
    let wrap_hotkey = DeserialiseHotkey(SHELL_CHORD_DELIM, shortcut.hotkey);
} {
    " '";
    wrap_hotkey.len(extra) => wrap_hotkey.push_into(extra, buffer);
    "')  ";
    shortcut.command.len() => for with_span in shortcut.command {
        buffer.push(with_span.source);
    };
    "\n";
});

pub struct Wrapper();
impl<'filestr, 'b> PreallocPush<'filestr, &'b ShortcutOwner<'filestr>> for Wrapper {
    sidebyside_len_and_push!(len, push_into (self: &Self, owner: &ShortcutOwner<'filestr>, buffer: 'filestr) {} {
        "#!/bin/sh\n";
        "case \"${1}\"\n";
        owner.to_iter().map(|s| 1 + len(s, SHELL_CONSTANTS)).sum::<usize>() => {
            let mut prefix = "in";
            owner.to_iter().filter(|s| !s.is_placeholder).for_each(|s| {
                buffer.push(prefix);
                prefix = ";;";
                push_into(s, SHELL_CONSTANTS, buffer);
            });
        };
        "*)  notify.sh \"invalid key combination ${1}\"; exit 1\n";
        "esac\n";
    });
}
