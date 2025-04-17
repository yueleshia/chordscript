// The third phase of compilation
// run: cargo test -- --nocapture

use std::cmp;
use std::mem;
use std::ops::Range;

use crate::parser::ShortcutOwner;
use crate::structs::{Chord, Cursor, Hotkey, Shortcut, WithSpan};

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
    Command(
        WithSpan<'filestr, Chord>,
        Shortcut<'parsemes, 'filestr>,
    ),
}

impl<'parsemes, 'filestr> Action<'parsemes, 'filestr> {
    pub fn key_trigger(&self) -> &WithSpan<'filestr, Chord> {
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
pub fn process<'parsemes, 'filestr>(
    shortcut_owner: &'parsemes ShortcutOwner<'filestr>,
) -> KeyspaceOwner<'parsemes, 'filestr> {
    let view = shortcut_owner.make_owned_sorted_view();
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
            into_partitions.clear();
            let mut partitions_cursor = Cursor(0);
            for base in to_process_list.iter() {
                partition_by_col_into(col, base, into_partitions);
                let range = partitions_cursor.move_to(into_partitions.len());
                if !into_partitions[range].is_empty() {
                    capacity.0 += 1;
                }
            }
            capacity.1 += into_partitions.len();
            mem::swap(to_process_list, into_partitions);
        }
        capacity
    };

    let owner = {
        let mut keyspaces = Vec::with_capacity(keyspace_capacity);
        let mut all_actions = Vec::with_capacity(action_capacity);
        let mut action_cursor = Cursor(0);
        to_process_list.clear();
        to_process_list.push(&view[..]);


        //println!("{:?}", deserialise);
        for col in 0..max_depth {
            into_partitions.clear();
            let mut partitions_cursor = Cursor(0);
            for base in to_process_list.iter() {
                partition_by_col_into(col, base, into_partitions);
                let range = partitions_cursor.move_to(into_partitions.len());
                let base_partitions = &into_partitions[range];
                for partition in base_partitions.iter() {
                    all_actions.push(partition_to_action(col, partition));
                }

                // If previous iteration only push a Action::Command, then 'base'
                // is partitioned into null (not pushed to 'all_actions')
                if !base_partitions.is_empty() {
                    // Pre-calculating 'max_depth' ensures >= one partition every 'col'
                    keyspaces.push(KeyspaceRef {
                        title: &base[0].hotkey[0..col],
                        actions: action_cursor.move_to(all_actions.len()),
                    });
                }
            }

            mem::swap(to_process_list, into_partitions);
        }

        //run: cargo run keyspaces -c $HOME/interim/hk/config.txt
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

/****************************************************************************
 * Control flow
 ****************************************************************************/
fn partition_to_action<'parsemes, 'filestr>(
    col: usize,
    partition: &[Shortcut<'parsemes, 'filestr>],
) -> Action<'parsemes, 'filestr> {
    let first_shortcut = &partition[0];
    let trigger = first_shortcut.hotkey[col].clone();
    if partition.len() == 1 && first_shortcut.hotkey.len() == col + 1 {
        Action::Command(trigger, first_shortcut.clone())
    } else {
        Action::SetState(&first_shortcut.hotkey[0..col + 1])
    }
}

fn partition_by_col_into<'a, 'owner, 'filestr>(
    col: usize,
    input: &'a [Shortcut<'owner, 'filestr>],
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
