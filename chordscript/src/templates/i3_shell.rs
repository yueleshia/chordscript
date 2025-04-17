use crate::parser::shortcuts::ShortcutOwner;
//use crate::structs::{Shortcut, Chord};
use crate::constants::{KEYCODES, MODIFIERS};
use crate::parser::keyspaces::{process, Action, Keyspace};
use crate::sidebyside_len_and_push;

use super::shellscript::{SHELL_CHORD_DELIM, SHELL_CONSTANTS};
use super::{DeserialiseChord, DeserialiseHotkey, PreallocPush, CHORD_MAX_PUSH_LEN};

//run: cargo test -- --nocapture

const TITLE_DELIM: &str = ";";
const TITLE_CONSTANTS: DeserialiseChord = DeserialiseChord {
    delim: "+",
    mod_to_str: &{
        let mut base = MODIFIERS;

        use crate::constants::Modifiers;
        base[Modifiers::Alt.id()] = "A";
        base[Modifiers::Ctrl.id()] = "C";
        base[Modifiers::Shift.id()] = "S";
        base[Modifiers::Super.id()] = "M";
        base
    },
    key_to_str: &KEYCODES,
};
const KEYBIND_CONSTANTS: DeserialiseChord = DeserialiseChord {
    delim: "+",
    mod_to_str: &{
        let mut base = MODIFIERS;

        use crate::constants::Modifiers;
        base[Modifiers::Alt.id()] = "Mod1";
        base[Modifiers::Ctrl.id()] = "Ctrl";
        base[Modifiers::Shift.id()] = "Shift";
        base[Modifiers::Super.id()] = "Mod4";
        base
    },
    key_to_str: &KEYCODES,
};

const UNUSED: DeserialiseChord = DeserialiseChord {
    delim: "",
    mod_to_str: &MODIFIERS,
    key_to_str: &KEYCODES,
};

impl<'filestr, 'b> PreallocPush<'filestr, DeserialiseChord> for Action<'b, 'filestr> {
    sidebyside_len_and_push!(len, push_into, self: &Self, _a: DeserialiseChord, buffer: 'filestr {} {
        "bindsym ";
        CHORD_MAX_PUSH_LEN => self.key_trigger().chord.push_into(KEYBIND_CONSTANTS, buffer);
        1 => match self {
            Action::SetState(_) => buffer.push(" mode \""),
            Action::Command(_, _) => buffer.push(" bindsym exec --no-startup-id \""),
        };

        match self {
            Action::SetState(title) => DeserialiseHotkey(TITLE_DELIM, title).len(TITLE_CONSTANTS) + 1,
            Action::Command(_, shortcut) => {
                1
                + 1
                + DeserialiseHotkey(SHELL_CHORD_DELIM, shortcut.hotkey).len(SHELL_CONSTANTS)
                + 1
            }
        } => match self {
            Action::SetState(title) => {
                //debug_assert!()
                DeserialiseHotkey(TITLE_DELIM, title).push_into(TITLE_CONSTANTS, buffer);
                buffer.push("\";\n");
            }
            Action::Command(_trigger, shortcut) => {
                buffer.push("shortcuts.sh");
                buffer.push(" '");
                DeserialiseHotkey(SHELL_CHORD_DELIM, shortcut.hotkey).push_into(SHELL_CONSTANTS, buffer);
                buffer.push("'\"; mode \"default\";\n");
            }
        };
    });
}

impl<'filestr, 'b, 'owner> PreallocPush<'filestr, DeserialiseChord>
    for Keyspace<'owner, 'b, 'filestr>
{
    sidebyside_len_and_push!(len, push_into, self: &Self, _a: DeserialiseChord, buffer: 'filestr {} {
        "\nmode \"";
            DeserialiseHotkey(TITLE_DELIM, self.title).len(TITLE_CONSTANTS) =>
                DeserialiseHotkey(TITLE_DELIM, self.title).push_into(TITLE_CONSTANTS, buffer);
            "\" {\n";
            self.actions.iter().map(|a| a.len(UNUSED) + 1).sum::<usize>() => for action in self.actions {
                buffer.push("  ");
                action.push_into(UNUSED, buffer);
            };
            "  bindsym Escape mode \"default\";\n";
        "}\n";
    });
}

pub struct Wrapper();
impl<'a, 'b> PreallocPush<'a, &'b ShortcutOwner<'a>> for Wrapper {
    sidebyside_len_and_push!(len, push_into, self: &Self, shortcut_owner: &ShortcutOwner<'a>, buffer: 'a {
        let owner = process(shortcut_owner);
        let mut iter = owner.to_iter();
        let modeless = iter.next().expect("DEV: We always expect `title: []` to exist");
    } {
        modeless.actions.iter().map(|action| action.len(UNUSED)).sum::<usize>() =>
            modeless.actions.iter().for_each(|action| action.push_into(UNUSED, buffer));
        "\n";
        iter.map(|keyspace| keyspace.len(UNUSED)).sum::<usize>() =>
            iter.for_each(|keyspace| keyspace.push_into(UNUSED, buffer));
    });
}

//sidebyside_len_and_push!(len, push_into, self: &Self, owner: &ShortcutOwner<'a>, buffer: 'a {} {
//    "#!/bin/sh\n";
//    "case \"${1}\"\n";
//    owner.shortcuts.len() + owner.to_iter().map(|s| s.len(DEFAULT_DESERIALISE)).sum::<usize>() => {
//        let mut prefix = "in";
//        owner.to_iter().filter(|s| !s.is_placeholder).for_each(|s| {
//            buffer.push(prefix);
//            prefix = ";;";
//            s.push_into(DEFAULT_DESERIALISE, buffer);
//        });
//    };
//    "*)  notify.sh \"invalid key combination ${1}\"; exit 1\n";
//    "esac\n";
//});
//}
//

//pub fn format<'a>(shortcut_owner: &ShortcutOwner<'a>) -> Vec<&'a str> {
//    let mut buffer = Vec::with_capacity(len(shortcut_owner, ()));
//    push_into(shortcut_owner, (), &mut buffer);
//    buffer
//}
