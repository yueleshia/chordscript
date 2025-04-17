use crate::constants::{KEYCODES, MODIFIERS};
use crate::deserialise::{TrimEscapeStrList, Print};
use crate::parser::{ShortcutOwner, ShortcutIter};
use crate::structs::{Chord, Shortcut, WithSpan};
use crate::{array, precalculate_capacity_and_build};

pub struct ListDebug<'parsemes, 'filestr>(pub &'parsemes ShortcutOwner<'filestr>);
pub struct ListPreview<'parsemes, 'filestr>(pub &'parsemes ShortcutOwner<'filestr>);
pub struct ListChord<'parsemes, 'filestr>(pub &'parsemes WithSpan<'filestr, Chord>);
struct ListIter<'parsemes, 'filestr>(ShortcutIter<'parsemes, 'filestr>);

const QUOTE: char = '\'';
const CANDIDATES: [char; 1] = ['\''];
const ESCAPE: [&str; 1] = ["'\\''"];

impl<'parsemes, 'filestr> Print for ListDebug<'parsemes, 'filestr> {
    precalculate_capacity_and_build!(self, buffer {} {
        13 => buffer.push_str("Placeholders\n");
        11 => buffer.push_str("==========\n");
        ListIter(self.0.to_placeholder_iter()).string_len() =>
            ListIter(self.0.to_placeholder_iter()).push_string_into(buffer);

        1 => buffer.push('\n');

        15 => buffer.push_str("Real Shortcuts\n");
        11 => buffer.push_str("==========\n");
        ListIter(self.0.to_iter()).string_len() =>
            ListIter(self.0.to_iter()).push_string_into(buffer);
    });
}

impl<'parsemes, 'filestr> Print for ListPreview<'parsemes, 'filestr> {
    precalculate_capacity_and_build!(self, buffer {} {
        ListIter(self.0.to_iter()).string_len() =>
            ListIter(self.0.to_iter()).push_string_into(buffer);
    });
}


impl<'parsemes, 'filestr> Print for ListIter<'parsemes, 'filestr> {
    precalculate_capacity_and_build!(self, buffer {} {
        self.0.clone().map(|Shortcut { hotkey, command }|
            1
            + array!(@len_join { hotkey } |> ListChord, " ; ")
            + 1
            + TrimEscapeStrList(QUOTE, &CANDIDATES, &ESCAPE, command).string_len()
            + 1
        ).sum::<usize>() => self.0.clone().for_each(|Shortcut { hotkey, command }| {
            buffer.push('|');
            array!(@push_join { hotkey } |> ListChord, " ; ", |> buffer);
            buffer.push('|');
            TrimEscapeStrList(QUOTE, &CANDIDATES, &ESCAPE, command).push_string_into(buffer);
            buffer.push('\n');
        });
    });
}

impl<'parsemes, 'filestr> Print for ListChord<'parsemes, 'filestr> {
    precalculate_capacity_and_build!(self, buffer {
        let Chord { key, modifiers } = self.0.data;
        let mut mod_iter = MODIFIERS.iter().enumerate()
            .filter(|(i, _)| modifiers & (1 << i) != 0);
        let space = ' ';
        debug_assert!(space.len_utf8() == 1);
        let first = mod_iter.next();
    } {
        // Process the first element separately to simulate a join()
        first.map(|(_, mod_str)| mod_str.len()).unwrap_or(0) =>
            if let Some((_, mod_str)) = first {
                buffer.push_str(mod_str);
            };
        mod_iter.map(|(_, mod_str)| mod_str.len() + 1).sum::<usize>() =>
            mod_iter.for_each(|(_, mod_str)| {
                buffer.push(space);
                buffer.push_str(mod_str);
            });


        // Then the key itself
        if key < KEYCODES.len() {
            KEYCODES[key].len() + if first.is_some() { 1 } else { 0 }
        } else {
            0
        } => if key < KEYCODES.len() {
             if first.is_some() {
                 buffer.push(space);
             }
             buffer.push_str(KEYCODES[key]);
         };
    });
}
