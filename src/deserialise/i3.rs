use crate::constants::KEYCODES;
use crate::deserialise::{DeserialisedChord, Print, TrimEscapeStrList};
use crate::keyspace::{Action, KeyspaceOwner};
use crate::structs::{Chord, WithSpan};
use crate::{array, define_buttons, precalculate_capacity_and_build};

pub struct I3<'keyspaces, 'parsemes, 'filestr>(pub &'keyspaces KeyspaceOwner<'parsemes, 'filestr>);
struct I3Action<'keyspaces, 'parsemes, 'filestr>(&'keyspaces Action<'parsemes, 'filestr>);

impl<'keyspaces, 'parsemes, 'filestr> Print for I3<'keyspaces, 'parsemes, 'filestr> {
    precalculate_capacity_and_build!(self, buffer {
        let mut iter = self.0.to_iter();
        // @TODO check if keyspace always has at least one
        let first = iter.next().unwrap();
        let mode = "\nmode \"";
        let mode_begin = "\" {\n";
        let mode_close = "}\n";
        let padding = "  ";
    } {
        array!(@len { first.actions } |> "", I3Action, "\n")
            => array!(@push { first.actions } |> "", I3Action, "\n", |> buffer);

        iter.map(|ks| mode.len()
            + array!(@len { ks.title } |> "", i3_wrap_title, "")
            + mode_begin.len()
            + array!(@len { ks.actions } |> padding, I3Action, "\n")
            + mode_close.len()
        ).sum::<usize>() => iter.for_each(|ks| {
            buffer.push_str(mode);
            #[cfg(debug_assertions)]
            debug_assert!(
                !array!(@to_string { ks.title } |> i3_wrap_title).contains('"')
            );

            array!(@push { ks.title } |> "", i3_wrap_title, "", |> buffer);
            buffer.push_str(mode_begin);
            array!(@push { ks.actions } |> padding, I3Action, "\n", |> buffer);
            buffer.push_str(mode_close);
        });
    });
}

impl<'keyspaces, 'parsemes, 'filestr> Print for I3Action<'keyspaces, 'parsemes, 'filestr> {
    precalculate_capacity_and_build!(self, buffer {
        let bind = "bindsym ";
        let set_mode = " mode \"";
        let exec = " exec --no-startup-id ";
        let close = "\"";
        let quote = '"';
        let candidates = &['"', '\\'];
        let escape = &["\\\\\"", "\\\\\\\\"];
        let trigger = i3_wrap_chord(self.0.key_trigger());
    } {
        match &self.0 {
            Action::SetState(title) =>
                bind.len()
                    + trigger.string_len()
                    + set_mode.len()
                    + array!(@len_join { title } |> i3_wrap_title, " ; ")
                    + close.len(),
            Action::Command(_, command) =>
                bind.len()
                    + trigger.string_len()
                    + exec.len()
                    + TrimEscapeStrList(quote, candidates, escape, command).string_len(),
        } => match &self.0 {
            Action::SetState(title) => {
                buffer.push_str(bind);
                trigger.push_string_into(buffer);

                #[cfg(debug_assertions)]
                debug_assert!(!trigger.to_string_custom().contains(quote));

                buffer.push_str(set_mode);
                array!(@push_join { title } |> i3_wrap_title, " ; ", |> buffer);
                buffer.push_str(close);
            }
            Action::Command(_, command) => {
                buffer.push_str(bind);
                trigger.push_string_into(buffer);
                buffer.push_str(exec);

                TrimEscapeStrList(quote, candidates, escape, command).push_string_into(buffer);
            }
        };
    });
}

//run: cargo test -- --nocapture
// Second ident is just the name of a test function
define_buttons!(@MODS I3_MODIFIERS test_hotkey_modifiers {
    Alt => "Mod1",
    Ctrl => "Ctrl",
    Shift => "Shift",
    Super => "Mod4",
});
// Second ident is just the name of a test function
define_buttons!(@MODS I3_TITLE_MODIFIERS test_title {
    Alt => "A",
    Ctrl => "C",
    Shift => "S",
    Super => "M",
});

fn i3_wrap_chord<'a, 'b>(chord: &'a WithSpan<'b, Chord>) -> DeserialisedChord<'a, 'b> {
    DeserialisedChord("+", chord, &KEYCODES, &I3_MODIFIERS)
}
fn i3_wrap_title<'a, 'b>(chord: &'a WithSpan<'b, Chord>) -> DeserialisedChord<'a, 'b> {
    DeserialisedChord("+", chord, &KEYCODES, &I3_TITLE_MODIFIERS)
}




#[test]
fn no_invalid_punctuation() {
    debug_assert!(KEYCODES.iter().find(|s| s.contains('"')).is_none());
    debug_assert!(KEYCODES.iter().find(|s| s.contains(';')).is_none());
    debug_assert!(I3_MODIFIERS.iter().find(|s| s.contains('"')).is_none());
    debug_assert!(I3_MODIFIERS.iter().find(|s| s.contains(';')).is_none());
    debug_assert!(I3_TITLE_MODIFIERS
        .iter()
        .find(|s| s.contains('"'))
        .is_none());
}
