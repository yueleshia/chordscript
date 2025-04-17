use crate::deserialise::{list_preview::ListChord, Print, TrimEscapeStrList};
use crate::parser::ShortcutOwner;
use crate::structs::Shortcut;
use crate::{array, precalculate_capacity_and_build};

pub struct Shellscript<'parsemes, 'filestr>(pub &'parsemes ShortcutOwner<'filestr>);
struct ShShortcut<'parsemes, 'filestr>(Shortcut<'parsemes, 'filestr>);

impl<'parsemes, 'filestr> Print for Shellscript<'parsemes, 'filestr> {
    precalculate_capacity_and_build!(self, buffer {
        let mut iter = self.0.to_iter();
    } {
        11 => buffer.push_str("#!/bin/sh\n");
        12 => buffer.push_str("case \"${1}\"\n");
        5 => buffer.push_str("  in ");
        iter.next().map(|x| ShShortcut(x).string_len()).unwrap_or(0) =>
            iter.next().map(|x| ShShortcut(x).push_string_into(buffer));

        iter.map(|x| ShShortcut(x).string_len()).sum::<usize>() =>
            iter.for_each(|x| ShShortcut(x).push_string_into(buffer));
        11 => buffer.push_str("*) echo yo\n");
        4 => buffer.push_str("esac\n");
    });
}

impl<'parsemes, 'filestr> Print for ShShortcut<'parsemes, 'filestr> {
    precalculate_capacity_and_build!(self, buffer {
        let Self(Shortcut { hotkey, command }) = self;
    } {
        1 => buffer.push('"');
        array!(@len_join { hotkey } |> ListChord, " ; ") => {
            array!(@push_join { hotkey } |> ListChord, " ; ", |> buffer);
        };
        2 => buffer.push_str("\")");
        //command.iter().map(WithSpan::as_str).map(str::len).sum::<usize>() =>
        //    command.iter().map(WithSpan::as_str).for_each(|s| buffer.push_str(s));
        TrimEscapeStrList(' ', &[], &[], command).string_len() =>
            TrimEscapeStrList(' ', &[], &[], command).push_string_into(buffer);
        6 => buffer.push_str("\n  ;; ");
    });
}
