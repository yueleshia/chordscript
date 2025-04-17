use crate::constants::{KEYCODES, MODIFIERS};
use crate::deserialise::{Print, TrimEscapeStrList};
use crate::parser::ShortcutOwner;
use crate::precalculate_capacity_and_build;
use crate::structs::{Chord, Shortcut};
use super::DeserialisedChord;

pub struct ListAll<'parsemes, 'filestr>(pub &'parsemes ShortcutOwner<'filestr>);
pub struct ListReal<'parsemes, 'filestr>(pub &'parsemes ShortcutOwner<'filestr>);
pub struct ListShortcut<'shortcuts, 'filestr>(pub Shortcut<'shortcuts, 'filestr>);
//pub struct ListIter<'parsemes, 'filestr: 'parsemes, T>(pub T)
//    where T:  Iterator<Item = Shortcut<'parsemes, 'filestr>> + Clone;

const QUOTE: char = '\'';
const CANDIDATES: [char; 1] = ['\''];
const ESCAPE: [&str; 1] = ["'\\''"];

// Mostly for the standard dump to STDOUT for debuging your config
impl<'parsemes, 'filestr> Print for ListAll<'parsemes, 'filestr> {
    precalculate_capacity_and_build!(self, buffer {
        let placeholders = self.0.to_iter().filter(|sc| sc.is_placeholder);
        let reals = self.0.to_iter().filter(|sc| !sc.is_placeholder);
    } {
        13 => buffer.push_str("Placeholders\n");
        11 => buffer.push_str("==========\n");
        placeholders.map(|sc| ListShortcut(sc).string_len()).sum::<usize>()
            => placeholders.for_each(|sc| ListShortcut(sc).push_string_into(buffer));

        1 => buffer.push('\n');

        15 => buffer.push_str("Real Shortcuts\n");
        11 => buffer.push_str("==========\n");
        reals.map(|sc| ListShortcut(sc).string_len()).sum::<usize>()
            => reals.for_each(|sc| ListShortcut(sc).push_string_into(buffer));
    });
}

// The same as 'ListAllUnsorted' but without the placeholders
impl<'parsemes, 'filestr> Print for ListReal<'parsemes, 'filestr> {
    precalculate_capacity_and_build!(self, buffer {
        let reals = self.0.to_iter().filter(|sc| !sc.is_placeholder);
    } {
        15 => buffer.push_str("Real Shortcuts\n");
        11 => buffer.push_str("==========\n");
        reals.map(|sc| ListShortcut(sc).string_len()).sum::<usize>()
            => reals.for_each(|sc| ListShortcut(sc).push_string_into(buffer));
    });
}

impl<'shortcuts, 'filestr> Print for ListShortcut<'shortcuts, 'filestr> {
    precalculate_capacity_and_build!(self, buffer {
        let Shortcut { hotkey, command, is_placeholder } = self.0;
        let mut hotkey = hotkey.iter();
        let first = hotkey.next().unwrap();
        let bar_type = if !is_placeholder { '|' } else { '!' };
    } {
        bar_type.len_utf8() => buffer.push(bar_type);
        wrap_chord(first).string_len() => wrap_chord(first).push_string_into(buffer);
        hotkey.map(|chord| wrap_chord(chord).string_len() + 3).sum::<usize>()
            => hotkey.for_each(|chord| {
                buffer.push_str(" ; ");
                wrap_chord(chord).push_string_into(buffer);
            });
        bar_type.len_utf8() => buffer.push(bar_type);
        1 => buffer.push(' ');

        TrimEscapeStrList(QUOTE, &CANDIDATES, &ESCAPE, command).string_len()
            => TrimEscapeStrList(QUOTE, &CANDIDATES, &ESCAPE, command).push_string_into(buffer);
        1 => buffer.push('\n');
    });
}

#[inline]
fn wrap_chord<'a, 'b>(chord: &'a Chord<'b>) -> DeserialisedChord<'a, 'b> {
    DeserialisedChord(" ", chord, &KEYCODES, &MODIFIERS)
}
