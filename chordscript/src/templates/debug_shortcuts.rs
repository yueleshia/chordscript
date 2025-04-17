use crate::parser::shortcuts::ShortcutOwner;
//use crate::structs::{Chord, ChordModifiers};
use crate::constants::{KEYCODES, MODIFIERS};
use crate::sidebyside_len_and_push;
use crate::structs::Shortcut;

use super::{DeserialiseChord, DeserialiseHotkey, PreallocPush};

//run: cargo test -- --nocapture

const DEBUG_CONSTANTS: DeserialiseChord = DeserialiseChord {
    delim: " ",
    mod_to_str: &MODIFIERS,
    key_to_str: &KEYCODES,
};
sidebyside_len_and_push!(len<'o, 's>, push_into<'o, 's>(shortcut: &Shortcut<'o, 's>, extra: DeserialiseChord, buffer: 's) {} {
    1 => if shortcut.is_placeholder { buffer.push("!") } else { buffer.push("|") };
    DeserialiseHotkey(" ; ", shortcut.hotkey).len(extra) => DeserialiseHotkey(" ; ", shortcut.hotkey).push_into(extra, buffer);
    1 => if shortcut.is_placeholder { buffer.push("!") } else { buffer.push("|") };
    " ";
    shortcut.command.len() => shortcut.command.iter().for_each(|with_span| buffer.push(with_span.source));
    "\n";
});

pub struct Wrapper();
impl<'a, 'b> PreallocPush<'a, &'b ShortcutOwner<'a>> for Wrapper {
    sidebyside_len_and_push!(len, push_into(self: &Self, owner: &ShortcutOwner<'a>, buffer: 'a) {} {
        owner.to_iter().map(|s| len(&s, DEBUG_CONSTANTS)).sum::<usize>() => {};
        "==== Placeholders ====\n";
        0 => owner.to_iter().filter(|s| s.is_placeholder).for_each(|s| push_into(&s, DEBUG_CONSTANTS, buffer));
        "==== Shortcuts ====\n";
        0 => owner.to_iter().filter(|s| !s.is_placeholder).for_each(|s| push_into(&s, DEBUG_CONSTANTS, buffer));
    });
}
