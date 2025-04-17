use crate::deserialise::{default_print_chord, Print, TrimEscapeStrList};
use crate::parser::ShortcutOwner;
use crate::structs::Shortcut;
use crate::{array, precalculate_capacity_and_build};

pub struct Shellscript<'parsemes, 'filestr>(pub &'parsemes ShortcutOwner<'filestr>);
struct ShShortcut<'parsemes, 'filestr>(Shortcut<'parsemes, 'filestr>);

impl<'parsemes, 'filestr> Print for Shellscript<'parsemes, 'filestr> {
    precalculate_capacity_and_build!(self, buffer {
        let mut iter = self.0.to_iter().filter(|s| !s.is_placeholder);
    } {
        "#!/bin/sh\n";
        "case \"${1}\"\n";
        "  in ";
        iter.next().map(|x| ShShortcut(x).string_len()).unwrap_or(0) =>
            iter.next().map(|x| ShShortcut(x).push_string_into(buffer));

        iter.map(|x| ShShortcut(x).string_len()).sum::<usize>() =>
            iter.for_each(|x| ShShortcut(x).push_string_into(buffer));
        "*) notify.sh \"key combination ${1}\"; exit 1\n";
        "esac\n";
    });
}

impl<'parsemes, 'filestr> Print for ShShortcut<'parsemes, 'filestr> {
    precalculate_capacity_and_build!(self, buffer {
        let Self(Shortcut { hotkey, command, .. }) = self;
    } {
        "\"";
        array!(@len_join { hotkey } |> default_print_chord, " ; ") => {
            array!(@push_join { hotkey } |> default_print_chord, " ; ", |> buffer);
        };
        "\")";
        //command.iter().map(WithSpan::as_str).map(str::len).sum::<usize>() =>
        //    command.iter().map(WithSpan::as_str).for_each(|s| buffer.push_str(s));
        TrimEscapeStrList(' ', &[], &[], command).string_len() =>
            TrimEscapeStrList(' ', &[], &[], command).push_string_into(buffer);
        "\n  ;; ";
    });
}
