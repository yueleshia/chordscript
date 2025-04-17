use crate::constants::{KEYCODES, MODIFIERS};
use crate::parser::{shortcuts::ShortcutOwner, Shortcut};
use crate::sidebyside_len_and_push;

use super::{Consumer, DeserialiseChord, DeserialiseHotkey, PreallocLen, PreallocPush};

//run: cargo test -- --nocapture

const DEBUG_CONSTANTS: DeserialiseChord = DeserialiseChord {
    delim: " ",
    mod_to_str: &MODIFIERS,
    key_to_str: &KEYCODES,
};
sidebyside_len_and_push!(shortcut_len, shortcut_pipe<U>(shortcut: &Shortcut, extra: DeserialiseChord, buffer: U) {} {
    1 => if shortcut.is_placeholder { buffer.consume("!") } else { buffer.consume("|") };
    DeserialiseHotkey(" ; ", shortcut.hotkey).len(extra) => DeserialiseHotkey(" ; ", shortcut.hotkey).pipe(extra, buffer);
    1 => if shortcut.is_placeholder { buffer.consume("!") } else { buffer.consume("|") };
    " ";
    shortcut.command.len() => shortcut.command.iter().for_each(|with_span| buffer.consume(with_span.source));
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
sidebyside_len_and_push!(len, pipe<U>(_me: (), owner: &ShortcutOwner, buffer: U) {} {
    owner.to_iter().map(|s| shortcut_len(&s, DEBUG_CONSTANTS)).sum::<usize>() => {};
    "==== Placeholders ====\n";
    0 => owner.to_iter().filter(|s| s.is_placeholder).for_each(|s| shortcut_pipe(&s, DEBUG_CONSTANTS, buffer));
    "==== Shortcuts ====\n";
    0 => owner.to_iter().filter(|s| !s.is_placeholder).for_each(|s| shortcut_pipe(&s, DEBUG_CONSTANTS, buffer));
});
