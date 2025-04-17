// The third phase of compilation
// run: cargo test -- --nocapture

use std::cmp;
use std::mem;
use std::ops::Range;

use super::shortcuts::ShortcutOwner;
use crate::structs::{Chord, Cursor, Hotkey, Shortcut};

/****************************************************************************
 * Token definitions
 ****************************************************************************/
// @TODO consider removing 'parsemes lifetime, i.e. copy the chord list
#[derive(Debug)]
struct KeyspaceRef<'parsemes, 'filestr> {
    title: Hotkey<'parsemes, 'filestr>,
    actions: Range<usize>,
}

#[derive(Debug)]
pub enum Action<'parsemes, 'filestr> {
    SetState(Hotkey<'parsemes, 'filestr>),
    Command(Chord<'filestr>, Shortcut<'parsemes, 'filestr>),
}

impl<'parsemes, 'filestr> Action<'parsemes, 'filestr> {
    pub fn key_trigger(&self) -> &Chord<'filestr> {
        match self {
            // Should always be at least one chord in title
            // There is no Action::SetState(&[])
            Action::SetState(title) => title.last().unwrap(),
            Action::Command(trigger, _) => trigger,
        }
    }
}

#[derive(Debug)]
pub struct KeyspaceOwner<'parsemes, 'filestr> {
    keyspaces: Vec<KeyspaceRef<'parsemes, 'filestr>>,
    all_actions: Vec<Action<'parsemes, 'filestr>>,
}

// Map Keyspace to Keyspace via Iterator interface with concrete types
impl<'parsemes, 'filestr> KeyspaceOwner<'parsemes, 'filestr> {
    pub fn to_iter<'a>(&'a self) -> KeyspaceIter<'a, 'parsemes, 'filestr> {
        KeyspaceIter {
            iter: self.keyspaces.iter(),
            owner: self,
        }
    }
}

//
#[derive(Debug)]
pub struct KeyspaceIter<'keyspaces, 'parsemes, 'filestr> {
    iter: std::slice::Iter<'keyspaces, KeyspaceRef<'parsemes, 'filestr>>,
    owner: &'keyspaces KeyspaceOwner<'parsemes, 'filestr>,
}

#[derive(Debug)]
pub struct Keyspace<'keyspaces, 'parsemes, 'filestr> {
    pub title: Hotkey<'parsemes, 'filestr>,
    pub actions: &'keyspaces [Action<'parsemes, 'filestr>],
}

impl<'keyspaces, 'parsemes, 'filestr> Iterator for KeyspaceIter<'keyspaces, 'parsemes, 'filestr> {
    type Item = Keyspace<'keyspaces, 'parsemes, 'filestr>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|KeyspaceRef { title, actions }| Keyspace {
                title,
                actions: &self.owner.all_actions[actions.start..actions.end],
            })
    }
}

/****************************************************************************
 * Syntax
 ****************************************************************************/
//run: cargo run keyspaces -c $XDG_CONFIG_HOME/rc/wm-shortcuts

// Filters out the placeholders
pub fn process<'parsemes, 'filestr>(shortcut_owner: &'parsemes ShortcutOwner<'filestr>)
-> KeyspaceOwner<'parsemes, 'filestr>
{
    let shortcut_count = shortcut_owner.shortcuts.len();
    let mut all_partition = Vec::with_capacity(shortcut_count);
    all_partition.extend(shortcut_owner.to_iter().filter(|s| !s.is_placeholder));
    let max_depth = all_partition
        .iter()
        .fold(0, |depth, shortcut| cmp::max(depth, shortcut.hotkey.len()));

    let todo = &mut Vec::with_capacity(shortcut_count); // aka. left
    let done = &mut Vec::with_capacity(shortcut_count); // aka. right
    todo.push(&all_partition[..]); // Push a slice containing everything

    // If a keyspace has > 1 action, then we are overestimating both
    let action_max_capacity = shortcut_owner.chords.len();
    // + 1 for the '[]' chord keyspace (i.e. just one chord press)
    let keyspace_max_capacity = action_max_capacity - shortcut_count + 1;

    let mut action_cursor = Cursor(0);
    let mut all_actions = Vec::with_capacity(action_max_capacity);
    let mut keyspaces = Vec::with_capacity(keyspace_max_capacity);
    for col in 0..max_depth {
        done.clear();
        // 'split_by_col_into' continually adds to 'done' (clear only happens
        // on the previous line, i.e. not within the for loop below)
        //
        // This prevents us from reprocessing 'right_partitions' (i.e. pushing
        // 'Action' enums) already in 'done' added by previous
        // 'split_by_col_into()' from 'left_partition' slices
        //
        // We cannot delete the previous 'right_partition' slices in 'done'
        // because 'swap()' at the end
        let mut right_cursor = Cursor(0);

        for left_partition in todo.iter() {
            // Populates 'done' with a refined (more split) list of partitions
            split_by_col_into(left_partition, col, done);

            //println!("col({}) {} {:?}..{}", col, todo.len(), right_cursor, done.len());
            // This deals with the right
            let refinement = &done[right_cursor.move_to(done.len())];
            for right_partition in refinement.iter() {
                let first = &right_partition[0];
                all_actions.push(if right_partition.len() == 1 && first.hotkey.len() == col + 1 {
                    Action::Command(first.hotkey[col].clone(), first.clone())
                } else {
                    Action::SetState(&first.hotkey[0..col + 1])
                });
            }

            // This deals with the left, must come after dealing with the right
            //
            // If previous iteration only push a Action::Command, then 'partition'
            // is partitioned into null (not pushed to 'all_actions')
            if !refinement.is_empty() {
                // Pre-calculating 'max_depth' ensures >= one partition every 'col'
                keyspaces.push(KeyspaceRef {
                    title: &left_partition[0].hotkey[0..col],
                    actions: action_cursor.move_to(all_actions.len()),
                });
            }

            //use crate::deserialise::{KeyspaceAction, ListShortcut};
            //use crate::deserialise::Print;
            //    .map(|a| KeyspaceAction(a).to_string_custom())
            //    .collect::<Vec<_>>()
            //    .join("\n")
            //);
            //println!("{}\n----", b.iter()
            //    .map(|a| ListShortcut(a.clone()).to_string_custom())
            //    .collect::<Vec<_>>()
            //    .join("\n"));

        }

        mem::swap(todo, done);
    }
    debug_assert_eq!(shortcut_count, todo.capacity());
    debug_assert_eq!(shortcut_count, done.capacity());
    debug_assert_eq!(keyspace_max_capacity, keyspaces.capacity());
    debug_assert_eq!(action_max_capacity, all_actions.capacity());

    KeyspaceOwner {
        keyspaces,
        all_actions,
    }
}

/****************************************************************************
 * Control flow
 ****************************************************************************/
// Assumes 'hotkey[0..col]' are all shared for the 'input'
// Splits the input list
// Partition an object by chord
fn split_by_col_into<'a, 'owner, 'filestr>(
    input: &'a [Shortcut<'owner, 'filestr>],
    col: usize,
    into_store: &mut Vec<&'a [Shortcut<'owner, 'filestr>]>,
) {
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
            "Sorting guarantee violated. Shortcuts from 'start_index'.. do not have enough chords"
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
