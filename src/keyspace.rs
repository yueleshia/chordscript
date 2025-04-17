// The third phase of compilation
//run: cargo test -- --nocapture

use std::cmp;
use std::mem;
use std::ops::Range;

use crate::parser::ShortcutOwner;
use crate::structs::{Chord, Cursor, Print, Shortcut, WithSpan};

/****************************************************************************
 * Token definitions
 ****************************************************************************/
#[derive(Debug)]
pub struct Keyspace<'parsemes, 'filestr> {
    title: &'parsemes [WithSpan<'filestr, Chord>],
    actions: Range<usize>,
}

#[derive(Debug)]
pub enum Action<'parsemes, 'filestr> {
    SetState(&'parsemes [WithSpan<'filestr, Chord>]),
    Command(
        WithSpan<'filestr, Chord>,
        &'parsemes [WithSpan<'filestr, ()>],
    ),
}

pub struct KeyspaceOwner<'parsemes, 'filestr> {
    keyspaces: Vec<Keyspace<'parsemes, 'filestr>>,
    all_actions: Vec<Action<'parsemes, 'filestr>>,
}

/****************************************************************************
 * Syntax
 ****************************************************************************/
pub fn process<'parsemes, 'filestr>(
    shortcut_owner: &'parsemes ShortcutOwner<'filestr>,
) -> KeyspaceOwner<'parsemes, 'filestr> {
    let view = shortcut_owner.make_owned_sorted_view();

    //let hotkeys: &[Hotkey] = &view;
    let shortcut_len = view.len();
    let max_depth = view
        .iter()
        .fold(0, |depth, shortcut| cmp::max(depth, shortcut.hotkey.len()));
    let to_process_list = &mut Vec::with_capacity(shortcut_len);
    let into_partitions = &mut Vec::with_capacity(shortcut_len);

    to_process_list.push(&view[..]);
    let (keyspace_capacity, action_capacity) = {
        let mut capacity = (0, 0);
        for col in 0..max_depth {
            for base in to_process_list.iter() {
                partition_by_col_into(col, base, into_partitions);
                capacity.1 += into_partitions.len();
                if !into_partitions.is_empty() {
                    capacity.0 += 1;
                }
            }
            mem::swap(to_process_list, into_partitions);
        }
        capacity
    };

    let owner = {
        let mut keyspaces = Vec::with_capacity(keyspace_capacity);
        let mut all_actions = Vec::with_capacity(action_capacity);
        let mut cursor = Cursor(0);
        to_process_list.clear();
        to_process_list.push(&view[..]);

        for col in 0..max_depth {
            for base in to_process_list.iter() {
                partition_by_col_into(col, base, into_partitions);
                into_partitions
                    .iter()
                    .for_each(|partition| all_actions.push(partition_to_action(col, partition)));

                // If previous iteration only push a Action::Command, then 'base'
                // is partitioned into null (not pushed to 'all_actions')
                if !into_partitions.is_empty() {
                    // Pre-calculating 'max_depth' ensures >= one partition every 'col'
                    keyspaces.push(Keyspace {
                        title: &base[0].hotkey[0..col],
                        actions: cursor.move_to(all_actions.len()),
                    });
                }
            }

            mem::swap(to_process_list, into_partitions);
        }
        debug_assert_eq!(shortcut_len, to_process_list.capacity());
        debug_assert_eq!(shortcut_len, into_partitions.capacity());
        debug_assert_eq!(keyspace_capacity, keyspaces.len());
        debug_assert_eq!(action_capacity, all_actions.len());

        KeyspaceOwner {
            keyspaces,
            all_actions,
        }
    };
    owner
}

//#[test]
//fn hello() {}

/****************************************************************************
 * Control flow
 ****************************************************************************/
fn partition_to_action<'parsemes, 'filestr>(
    col: usize,
    partition: &[Shortcut<'parsemes, 'filestr>],
) -> Action<'parsemes, 'filestr> {
    let first_shortcut = &partition[0];
    if partition.len() == 1 && first_shortcut.hotkey.len() == col + 1 {
        let chord = first_shortcut.hotkey[col].clone();
        Action::Command(chord, first_shortcut.command)
    } else {
        let shared_chord = &first_shortcut.hotkey[0..col + 1];
        Action::SetState(shared_chord)
    }
}

fn partition_by_col_into<'a, 'owner, 'filestr>(
    col: usize,
    input: &'a [Shortcut<'owner, 'filestr>],
    into_store: &mut Vec<&'a [Shortcut<'owner, 'filestr>]>,
) {
    into_store.clear();
    let first = input
        .iter()
        .enumerate()
        .find(|(_, shortcut)| shortcut.hotkey.get(col).is_some());

    // Trim off the hotkeys that are not of length >= col
    if let Some((start_index, start_shortcut)) = first {
        let partition = &input[start_index..];
        debug_assert!(
            partition
                .iter()
                .all(|shortcut| shortcut.hotkey.get(col).is_some()),
            "Sorting guaranteed violated. Shortcuts from 'start_index'.. do not have enough chords"
        );

        let mut start_chord = &start_shortcut.hotkey[col];
        let mut cursor = Cursor(start_index);
        // Skip 'start_shortcut'
        for (i, shortcut) in partition.iter().enumerate().skip(1) {
            //println!("")
            let close_chord = &shortcut.hotkey[col];
            if start_chord != close_chord {
                start_chord = close_chord;
                into_store.push(&partition[cursor.move_to(i)]);
            }
        }
        let range = cursor.move_to(partition.len());
        // Always going to push at least '&[start_shortcut]'
        debug_assert!(!range.is_empty());
        into_store.push(&partition[range]);
    } // else do not push hotkeys without enough chords
}

/****************************************************************************
 * Printing
 ****************************************************************************/
pub fn debug_print_keyspace_owner(
    KeyspaceOwner {
        keyspaces,
        all_actions,
    }: &KeyspaceOwner,
) {
    for Keyspace { title, actions } in keyspaces {
        let actions = &all_actions[actions.start..actions.end];
        let len = actions.len();
        let action_string = actions
            .iter()
            .map(|action| match action {
                Action::SetState(name) => format!(
                    "set state {}",
                    name.to_string_custom(),
                ),
                Action::Command(chord, command) => format!(
                    "{} -> {:?}",
                    chord.to_string_custom(),
                    command
                        .iter()
                        .map(|with_span| with_span.as_str())
                        .collect::<Vec<_>>()
                        .join("")
                ),
            })
            .collect::<Vec<_>>()
            .join("\n  ");

        println!(
            "state {} {:?} {{\n  {}\n}}\n",
            len,
            title.to_string_custom(),
            action_string,
        );
    }
}
