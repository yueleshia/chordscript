//run: cargo test -- --nocapture
// run: cargo build; time cargo run -- debug-shortcuts -c $XDG_CONFIG_HOME/rc/wm-shortcuts keyspace-list

use crate::constants::KEYCODES;
use crate::constants::MODIFIERS;
use crate::errors::parser as errors;
use crate::lexer::{LexOutput, Lexeme, PostLexEntry};
use crate::reporter::MarkupError;
use crate::structs::{Chord, Shortcut, WithSpan};

use std::ops::Range;
type Output<T> = Result<T, MarkupError>;

#[derive(Debug)]
pub struct ShortcutOwner<'filestr> {
    pub chords: Vec<Chord<'filestr>>,
    scripts: Vec<WithSpan<'filestr, ()>>,
    pub shortcuts: Vec<ShortcutPointer>,
}

#[derive(Clone, Debug)]
pub struct ShortcutPointer {
    is_placeholder: bool,
    head: Range<usize>,
    body: Range<usize>,
}

impl<'filestr> ShortcutOwner<'filestr> {
    fn sort(&mut self) {
        let shortcuts = &mut self.shortcuts;
        let chords = &self.chords;
        shortcuts.sort_by(|a, b| {
            let head1 = &chords[a.head.start..a.head.end];
            let head2 = &chords[b.head.start..b.head.end];
            head1.cmp(head2)
        });
    }

    pub fn to_iter<'me>(&'me self) -> impl Iterator<Item = Shortcut<'me, 'filestr>> {
        self.shortcuts.iter().map(move |pointer| Shortcut {
            is_placeholder: pointer.is_placeholder,
            hotkey: &self.chords[pointer.head.start..pointer.head.end],
            command: &self.scripts[pointer.body.start..pointer.body.end],
        })
    }
}

/******************************************************************************
 * Main Parse Workflow
 ******************************************************************************/
// Sorting is necessary for 'verify_no_overlap()'
pub fn parse(input: LexOutput) -> Output<ShortcutOwner> {
    let mut owner = parse_main(input)?;
    owner.sort();
    verify_no_overlap(&owner)?;
    Ok(owner)
}

// Although sorting is necessary for `verify_no_overlap()`, this will return
// the original order of the shortcuts. Good for debugging.
pub fn parse_unsorted(input: LexOutput) -> Output<ShortcutOwner> {
    let mut owner = parse_main(input)?;
    let original_order = owner.shortcuts.clone();
    owner.sort();
    verify_no_overlap(&owner)?;
    Ok(ShortcutOwner {
        chords: owner.chords,
        scripts: owner.scripts,
        shortcuts: original_order,
    })
}

fn parse_main(input: LexOutput) -> Output<ShortcutOwner> {
    //println!("{:#?}", input.entry_stats);
    let (permutation_count, head_aggregate_size, body_aggregate_size) =
        input.entry_stats.iter().fold((0, 0, 0), |(a, b, c), s| {
            (a + s.permutations, b + s.head_size, c + s.body_size)
        });

    // Allocate the memory
    let mut chords = vec![Chord::new(input.original); head_aggregate_size];
    let mut scripts = vec![
        WithSpan {
            data: (),
            context: input.original,
            source: &input.original[0..0],
        };
        body_aggregate_size
    ];
    let mut shortcuts = vec![
        ShortcutPointer {
            is_placeholder: false,
            head: 0..0,
            body: 0..0,
        };
        permutation_count
    ];
    let mut slice_holder = Vec::with_capacity(input.entry_stats.len());
    //println!("{} {:?}\n{} {:?}\n{} {:?}", chords.len(), chords, scripts.len(), scripts, shortcuts.len(), shortcuts);

    // Partition the memory into 'ThreadLocalStorage' chunks
    //
    // If I were to parallelize this, this is what I would want to do
    // I cannot figure out how to do with this iterators properly
    // Split the 'chord_partition' into indepedent mutable slices for iteration
    let mut chord_partitions = chords.as_mut_slice();
    let mut script_partitions = scripts.as_mut_slice();
    let mut shortcut_partitions = shortcuts.as_mut_slice();
    let mut head_index = 0;
    let mut body_index = 0;
    for stats in &input.entry_stats {
        let head = chord_partitions.split_at_mut(stats.head_size);
        chord_partitions = head.1;
        let head = head.0;

        let body = script_partitions.split_at_mut(stats.body_size);
        script_partitions = body.1;
        let body = body.0;

        let shortcut = shortcut_partitions.split_at_mut(stats.permutations);
        shortcut_partitions = shortcut.1;
        let shortcut = shortcut.0;

        slice_holder.push(ThreadLocalStorage {
            head_index,
            body_index,
            head,
            body,
            shortcut,
        });
        head_index += stats.head_size;
        body_index += stats.body_size;
    }

    // Parse the lexeme stream
    let lexemes = &input.lexemes;
    input
        .entry_stats
        .iter()
        .zip(slice_holder)
        .map(|(stats, mut storage)| {
            let head = &lexemes[stats.head..stats.body];
            let body = &lexemes[stats.body..stats.tail];
            parse_head_lex_into_chords(&mut storage, stats, head)?;
            parse_body_lex_into_scripts(&mut storage, stats, body);
            // Do not actually care about the 'Ok()' case, mostly for debug
            Ok((head, body))
        })
        //.map(|result| {
        //    if let Ok(a) = result {
        //        println!("{:?}\n{:?}\n----", a.0,a.1);
        //    }
        //    result
        //})
        .find(Result::is_err)
        .unwrap_or(Ok((&[], &[])))?;

    //chords.iter().for_each(|a| println!("{:?}", a.sources));
    //scripts.iter().for_each(|a| println!("{:?}", a.sources));
    //shortcuts.iter().for_each(|a| println!("{:?}", a));

    //            let body = body.iter().filter_map(|l| body_lexeme_to_str(i, l)).collect::<Vec<_>>().join("");

    debug_assert_eq!(chords.len(), head_aggregate_size);
    debug_assert_eq!(scripts.len(), body_aggregate_size);
    debug_assert_eq!(shortcuts.len(), permutation_count);
    Ok(ShortcutOwner {
        chords,
        scripts,
        shortcuts,
    })
}

// Verify that all hotkeys are accessible (and no duplicates)
// e.g. 'super + a' and 'super + a; super + b' cannot be used at the same time
fn verify_no_overlap(sorted_shortcuts: &ShortcutOwner) -> Output<()> {
    // Check 'sorted_shortcuts' is actually sorted
    debug_assert!(
        {
            let mut temp = sorted_shortcuts.to_iter().collect::<Vec<_>>();
            temp.sort_by(|a, b| a.hotkey.cmp(b.hotkey));
            // We didn't impl 'partial_eq' for 'Shortcut' so do it manually
            sorted_shortcuts
                .to_iter()
                .zip(temp)
                .fold(true, |_, (a, b)| {
                    a.hotkey == b.hotkey && a.command.as_ptr() == b.command.as_ptr()
                })
        },
        "DEV: You forgot to sort the array"
    );

    let mut iter = sorted_shortcuts.to_iter();
    if let Some(first) = iter.next() {
        iter.try_fold(first, |prev, curr| {
            let prev_len = prev.hotkey.len();
            let curr_len = curr.hotkey.len();
            //println!("{} {}", prev_len, curr_len);
            //println!("{:?}", curr.hotkey[0].sources);//, curr.hotkey[1].sources);
            if prev_len <= curr_len && prev.hotkey == &curr.hotkey[0..prev_len] {
                let prev_hotkey = prev
                    .hotkey
                    .iter()
                    .map(|chord| chord.sources.join(" "))
                    .collect::<Vec<_>>()
                    .join(" ; ");

                let curr_hotkey = prev
                    .hotkey
                    .iter()
                    .map(|chord| chord.sources.join(" "))
                    .collect::<Vec<_>>()
                    .join(" ; ");
                //MarkupError::from_str()
                todo!(
                    "have not created span errors yet\n\
                    {}\nconflicts with\n{}",
                    prev_hotkey,
                    curr_hotkey
                );
            } else {
                Ok(curr)
            }
        })?;
    }
    Ok(())
}

/******************************************************************************
 * Head and Body Parse
 ******************************************************************************/
// If we  were to parallelize or rayon this, this setup is probably useful
#[derive(Debug)]
struct ThreadLocalStorage<'a, 'filestr> {
    head_index: usize,
    body_index: usize,
    head: &'a mut [Chord<'filestr>],
    body: &'a mut [WithSpan<'filestr, ()>],
    shortcut: &'a mut [ShortcutPointer],
}

#[inline]
fn parse_head_lex_into_chords<'filestr>(
    storage: &mut ThreadLocalStorage<'_, 'filestr>,
    stats: &PostLexEntry,
    lexemes: &[Lexeme<'filestr>],
) -> Output<()> {
    let mut index = 0;
    let mut keys_added = 0;
    for i in 0..stats.permutations {
        let start = index;

        for lexeme in lexemes {
            let chord = &mut storage.head[index];
            match lexeme {
                Lexeme::Key(k) => {
                    chord.add(k, keys_added)?;
                    keys_added += 1;
                }
                Lexeme::HChoice(choice, k) if *choice == i => {
                    chord.add(k, keys_added)?;
                    keys_added += 1;
                }
                Lexeme::ChordDelimH(_) => {
                    index += 1;
                    keys_added = 0;
                }
                Lexeme::ChordDelimHC(choice, _) if *choice == i => {
                    index += 1;
                    keys_added = 0;
                }
                Lexeme::HChoice(_, _) | Lexeme::ChordDelimHC(_, _) => {}
                _ => unreachable!("{:?}", lexeme),
            }
        }
        let base = storage.head_index;
        storage.shortcut[i].is_placeholder = stats.is_placeholder;
        storage.shortcut[i].head = (base + start)..(base + index);
    }
    Ok(())
}

#[inline]
fn parse_body_lex_into_scripts<'a, 'filestr>(
    storage: &mut ThreadLocalStorage<'a, 'filestr>,
    stats: &PostLexEntry,
    lexemes: &[Lexeme<'filestr>],
) {
    let mut index = 0;
    for i in 0..stats.permutations {
        let start = index;
        for lexeme in lexemes {
            let frag = &mut storage.body[index];
            match lexeme {
                Lexeme::Literal(s) => {
                    frag.source = s;
                    index += 1;
                }
                Lexeme::BChoice(choice, s) if *choice == i => {
                    frag.source = s;
                    index += 1;
                }
                Lexeme::BChoice(_, _) => {}
                _ => unreachable!("{:?}", lexeme),
            }
        }
        let base = storage.body_index;
        storage.shortcut[i].body = (base + start)..(base + index);
    }
}

impl<'filestr> Chord<'filestr> {
    fn add(&mut self, key: &'filestr str, keys_added: usize) -> Output<()> {
        //println!("{}", self);
        if let Some(m) = MODIFIERS.iter().position(|m| *m == key) {
            let as_flag = 1 << m;
            if self.modifiers & as_flag == 0 {
                self.modifiers |= as_flag;
                self.sources[keys_added + 1] = key;
                Ok(())
            } else {
                Err(MarkupError::from_str(
                    self.context,
                    key,
                    "Modifier already used".into(),
                ))
            }
        //println!("{}", as_flag);
        } else if let Some(k) = KEYCODES.iter().position(|k| *k == key) {
            if self.key == KEYCODES.len() {
                self.key = k;
                self.sources[0] = key;
                Ok(())
            } else {
                Err(MarkupError::from_str(
                    self.context,
                    key,
                    "Key already used".into(),
                ))
            }
        } else {
            Err(MarkupError::from_str(
                self.context,
                key,
                errors::INVALID_KEY.to_string(),
            ))
        }
    }
}
