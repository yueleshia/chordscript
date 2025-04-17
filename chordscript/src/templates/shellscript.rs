use crate::constants::{KEYCODES, MODIFIERS};
use crate::parser::{shortcuts::ShortcutOwner, Shortcut};
use crate::sidebyside_len_and_push;

use super::{Consumer, DeserialiseChord, DeserialiseHotkey, PreallocLen, PreallocPush};

//run: cargo test -- --nocapture

//// @TODO: Extract chord into WithSpan<Chord>, would remove one lifetime
//struct DeserialisedChord {
//    delim: &'static str,
//    key: usize,
//    modifiers: ChordModifiers,
//    mod_to_str: &'static [&'static str],
//    key_to_str: &'static [&'static str],
//}

pub(crate) const SHELL_CONSTANTS: DeserialiseChord = DeserialiseChord {
    delim: " ",
    mod_to_str: &MODIFIERS,
    key_to_str: &KEYCODES,
};
pub(crate) const SHELL_CHORD_DELIM: &str = " ; ";

sidebyside_len_and_push!(shortcut_len, shortcut_pipe<U>
(shortcut: Shortcut, extra: DeserialiseChord, buffer: U) {
    let wrap_hotkey = DeserialiseHotkey(SHELL_CHORD_DELIM, shortcut.hotkey);
} {
    " '";
    wrap_hotkey.len(extra) => wrap_hotkey.pipe(extra, buffer);
    "')  ";
    shortcut.command.len() => for with_span in shortcut.command {
        buffer.consume(with_span.source);
    };
    "\n";
});

pub struct Wrapper();
impl PreallocLen<&ShortcutOwner<'_>> for Wrapper {
    fn len(&self, owner: &ShortcutOwner<'_>) -> usize {
        len((), owner)
    }
}
impl<U: Consumer> PreallocPush<&ShortcutOwner<'_>, U> for Wrapper {
    fn pipe(&self, owner: &ShortcutOwner<'_>, buffer: &mut U) {
        pipe((), owner, buffer)
    }
}
sidebyside_len_and_push!(len, pipe<U>(_a: (), owner: &ShortcutOwner, buffer: U) {} {
    "#!/bin/sh\n";
    "case \"${1}\"\n";
    owner.to_iter().map(|s| 1 + shortcut_len(s, SHELL_CONSTANTS)).sum::<usize>() => {
        let mut prefix = "in";
        owner.to_iter().filter(|s| !s.is_placeholder).for_each(|s| {
            buffer.consume(prefix);
            shortcut_pipe(s, SHELL_CONSTANTS, buffer);
            prefix = ";;";
        });
    };
    "*)  notify.sh \"invalid key combination ${1}\"; exit 1\n";
    "esac\n";
});
