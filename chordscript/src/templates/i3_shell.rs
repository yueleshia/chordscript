use crate::constants::{KEYCODES, MODIFIERS};
use crate::parser::keyspaces::{process, Action, Keyspace};
use crate::parser::shortcuts::ShortcutOwner;
use crate::sidebyside_len_and_push;

use super::shellscript::{SHELL_CHORD_DELIM, SHELL_CONSTANTS};
use super::{
    Consumer, DeserialiseChord, DeserialiseHotkey, PreallocLen, PreallocPush, CHORD_MAX_PUSH_LEN,
};

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

//struct WrapAction<'a, 'b>(Action<'a, 'b>);
impl<'a, 'b> PreallocLen<DeserialiseChord> for Action<'a, 'b> {
    fn len(&self, extra: DeserialiseChord) -> usize {
        action_len(self, extra)
    }
}
impl<'a, 'b, U: Consumer> PreallocPush<DeserialiseChord, U> for Action<'a, 'b> {
    fn pipe(&self, extra: DeserialiseChord, buffer: &mut U) {
        action_pipe(self, extra, buffer)
    }
}

sidebyside_len_and_push!(action_len, action_pipe<U>(me: &Action, _a: DeserialiseChord, buffer: U) {} {
    "bindsym ";
    CHORD_MAX_PUSH_LEN => me.key_trigger().chord.pipe(KEYBIND_CONSTANTS, buffer);
    1 => match me {
        Action::SetState(_) => buffer.consume(" mode \""),
        Action::Command(_, _) => buffer.consume(" bindsym exec --no-startup-id \""),
    };

    match me {
        Action::SetState(title) => DeserialiseHotkey(TITLE_DELIM, title).len(TITLE_CONSTANTS) + "\";\n".len(),
        Action::Command(_, shortcut) => {
            "shortcuts.sh".len()
            + 1
            + DeserialiseHotkey(SHELL_CHORD_DELIM, shortcut.hotkey).len(SHELL_CONSTANTS)
            + 1
        }
    } => match me {
        Action::SetState(title) => {
            //debug_assert!()
            DeserialiseHotkey(TITLE_DELIM, title).pipe(TITLE_CONSTANTS, buffer);
            buffer.consume("\";\n");
        }
        Action::Command(_trigger, shortcut) => {
            buffer.consume("shortcuts.sh");
            buffer.consume(" '");
            DeserialiseHotkey(SHELL_CHORD_DELIM, shortcut.hotkey).pipe(SHELL_CONSTANTS, buffer);
            buffer.consume("'\"; mode \"default\";\n");
        }
    };
});

impl<'a, 'b, 'c> PreallocLen<DeserialiseChord> for Keyspace<'a, 'b, 'c> {
    fn len(&self, extra: DeserialiseChord) -> usize {
        keyspace_len(self, extra)
    }
}
impl<'a, 'b, 'c, U: Consumer> PreallocPush<DeserialiseChord, U> for Keyspace<'a, 'b, 'c> {
    fn pipe(&self, extra: DeserialiseChord, buffer: &mut U) {
        keyspace_pipe(self, extra, buffer)
    }
}
sidebyside_len_and_push!(keyspace_len, keyspace_pipe<U>(me: &Keyspace, _a: DeserialiseChord, buffer: U) {} {
    "\nmode \"";
        DeserialiseHotkey(TITLE_DELIM, me.title).len(TITLE_CONSTANTS) =>
            DeserialiseHotkey(TITLE_DELIM, me.title).pipe(TITLE_CONSTANTS, buffer);
        "\" {\n";
        me.actions.iter().map(|a| a.len(UNUSED) + 1).sum::<usize>() => for action in me.actions {
            buffer.consume("  ");
            action.pipe(UNUSED, buffer);
        };
        "  bindsym Escape mode \"default\";\n";
    "}\n";
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
sidebyside_len_and_push!(len, pipe<U>(_me: (), shortcut_owner: &ShortcutOwner, buffer: U) {
    let owner = process(shortcut_owner);
    let mut iter = owner.to_iter();
    let modeless = iter.next().expect("DEV: We always expect `title: []` to exist");
} {
    modeless.actions.iter().map(|action| action.len(UNUSED)).sum::<usize>() =>
        modeless.actions.iter().for_each(|action| action.pipe(UNUSED, buffer));
    "\n";
    iter.map(|keyspace| keyspace.len(UNUSED)).sum::<usize>() =>
        iter.for_each(|keyspace| keyspace.pipe(UNUSED, buffer));
});
