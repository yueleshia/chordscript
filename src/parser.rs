use std::cmp;
use std::mem;
use std::ops::Range;

use crate::constants::KEYCODES;
use crate::define_syntax;
use crate::lexer::{BodyLexeme, HeadLexeme, Lexeme, LexemeOwner};
use crate::reporter::MarkupError;
use crate::structs::{Chord, Cursor, Shortcut, WithSpan};

//run: cargo test -- --nocapture

/****************************************************************************
 * Structs
 ****************************************************************************/
#[derive(Debug)]
pub struct HotkeyOwner<'filestr> {
    hotkeys: Vec<(Range<usize>, Range<usize>)>,
    chords: Vec<Chord>,
    scripts: Vec<&'filestr str>,
}

// @TODO Add verification for to catch e.g 'a;b' and 'a;b;c' hotkeys
// Choose this API to meaningfully separate a sorted and original-order view
// Also keeps the fields in 'HotkeyOwner' private
impl<'filestr> HotkeyOwner<'filestr> {
    // Renders the hotkeys from 'Range<usize>' into 'Hotkey'
    pub fn make_owned_view<'owner>(&'owner self) -> Vec<Shortcut<'owner, 'filestr>> {
        let mut hotkeys = Vec::with_capacity(self.hotkeys.len());
        hotkeys.extend(self.hotkeys.iter().map(|(head, body)| Shortcut {
            hotkey: &self.chords[head.start..head.end],
            command: &self.scripts[body.start..body.end],
        }));
        hotkeys
    }
}

/****************************************************************************
 * Syntax
 ****************************************************************************/
pub fn process<'filestr>(
    lexeme_stream: &'filestr LexemeOwner<'filestr>,
) -> Result<HotkeyOwner<'filestr>, MarkupError> {
    let mut rendered_head_capacity = 0;
    let mut rendered_body_capacity = 0;
    let mut hotkey_capacity = 0;
    let mut partition_max = 0;
    for lexeme in lexeme_stream.to_iter() {
        let (head_data, body_data) = parse_lexeme(&lexeme, &mut None, &mut None)?;

        //println!("Sections: {}", head_data.partition_count);
        //println!("Permutations: {}", head_data.space.permutations);
        //println!("Space: {}", head_data.space.items);

        rendered_head_capacity += head_data.space.items;
        rendered_body_capacity += body_data.space.items;

        hotkey_capacity += head_data.space.permutations;
        partition_max = cmp::max(partition_max, head_data.partition_count);
        partition_max = cmp::max(partition_max, body_data.partition_count);

        //println!("Body {:?}", body_data.space);
        //println!("Body {:?}", lexeme.body);
        assert!(head_data.space.permutations >= body_data.space.permutations);
        assert!(head_data.space.permutations >= body_data.space.permutations);
    }

    let parsemes = {
        //let (arrangement, permutations) = &mut Permutations::new(partition_max);
        //permutations.link_to(arrangement);
        let head_generator = &mut Permutations::new(partition_max);
        let body_generator = &mut Permutations::new(partition_max);
        let mut temp = HotkeyOwner {
            hotkeys: Vec::with_capacity(hotkey_capacity),
            chords: Vec::with_capacity(rendered_head_capacity),
            scripts: Vec::with_capacity(rendered_body_capacity),
        };
        for hotkey_lexeme in lexeme_stream.to_iter() {
            head_generator.reset();
            body_generator.reset();

            // Head
            let (head_data, _) = parse_lexeme(
                &hotkey_lexeme,
                &mut Some(head_generator),
                &mut Some(body_generator),
            )?;

            // Build owner
            let arrangement_count = head_data.space.permutations;
            for i in 0..arrangement_count {
                let head = head_generator.generate_arrangement(i);
                let body = body_generator.generate_arrangement(i);
                temp.push_arrangement(&hotkey_lexeme, head, body)?;
            }

            //println!("{:?} {:?}", head_generator, body_generator);
        }
        // @TODO check capacities match lengths
        temp
    };

    Ok(parsemes)
}

define_syntax! {
    parse | state: State
        ! metadata: &mut Metadata, index: usize, is_push: &mut Option<&mut Permutations>,
        (lexeme: Either)
    | -> (),
    Loop {
        Either::H(WithSpan(HeadLexeme::ChordDelim, _, _)) => metadata.partition_width += 1;
        Either::B(WithSpan(BodyLexeme::Section, _, _)) => metadata.partition_width += 1;

        Either::H(WithSpan(HeadLexeme::ChoiceBegin, _, _))
        | Either::B(WithSpan(BodyLexeme::ChoiceBegin, _, _)) => {
            if let Some(generator) = is_push {
                generator.push_digit_weight(1, index);
                // Skip HeadLexeme::ChoiceBegin
                generator.cursor.move_to(index + 1);
            }
            metadata.space.calc_space(1, metadata.partition_width);

            metadata.choice_count = 0;
            metadata.partition_width = 0;
            metadata.partition_count += 1;
        };

        Either::H(WithSpan(HeadLexeme::ChoiceDelim, _, _))
        | Either::B(WithSpan(BodyLexeme::ChoiceDelim, _, _)) =>
            metadata.choice_count += 1;

        Either::H(WithSpan(HeadLexeme::ChoiceClose, _, _))
        | Either::B(WithSpan(BodyLexeme::ChoiceClose, _, _))  => {
            if let Some(generator) = is_push {
                generator.push_digit_weight(metadata.choice_count + 1, index);
                // Skip HeadLexeme::ChoiceClose
                generator.cursor.move_to(index + 1);
            }
            metadata.space.calc_space(metadata.choice_count + 1, metadata.partition_width);
            //metadata.choice_count = 0; // Only needed if we reset
            metadata.partition_width = 0;
            metadata.partition_count += 1;
        };
        _ => {};
    }

    End {
        _ => {
            metadata.space.calc_space(1, metadata.partition_width + 1);
            metadata.partition_count += 1;
            // NOTE: index for State::End is the rindex (+ 1 of normal)
            if let Some(generator) = is_push {
                // Include till end
                generator.push_digit_weight(1, index);
                debug_assert_eq!(metadata.space.permutations, generator.permutations());
            }
        };
    }
}

//hotkeys: Vec<(Range<usize>, Range<usize>)>,
//chords: Vec<Chord>,
//scripts: Vec<&'filestr str>,
impl<'filestr> HotkeyOwner<'filestr> {
    fn push_arrangement(
        &mut self,
        hotkey_lexeme: &Lexeme<'_, 'filestr>,
        head_arrangement: (&[usize], &[Range<usize>]),
        body_arrangement: (&[usize], &[Range<usize>]),
    ) -> Result<(), MarkupError> {
        //self.head_arrange
        debug_assert_eq!(head_arrangement.0.len(), head_arrangement.1.len());
        debug_assert_eq!(body_arrangement.0.len(), body_arrangement.1.len());

        let (head_begin, body_begin) = (self.chords.len(), self.scripts.len());
        let mut chord = Chord::new();
        head_arrangement
            .1
            .iter()
            .map(|range| &hotkey_lexeme.head[range.start..range.end])
            .zip(head_arrangement.0.iter())
            .map(|(section, choice)| {
                section
                    .split(|l| matches!(l, WithSpan(HeadLexeme::ChoiceDelim, _, _)))
                    .nth(*choice)
                    .expect("Unreachable: Head choice index exceeded the current section")
            })
            .try_for_each(|key_sequence| {
                key_sequence.iter().try_for_each(|key| match key {
                    WithSpan(HeadLexeme::ChordDelim, _, _) => {
                        self.chords.push(mem::replace(&mut chord, Chord::new()));
                        Ok(())
                    }
                    WithSpan(HeadLexeme::Mod(_), _, _) => chord.add(key),
                    WithSpan(HeadLexeme::Key(_), _, _) => chord.add(key),
                    WithSpan(HeadLexeme::Blank, _, _) => Ok(()),
                    _ => unreachable!("Parser for head failed to catch {:?}", key),
                })
            })?;
        self.chords.push(chord);

        body_arrangement
            .1
            .iter()
            .map(|range| &hotkey_lexeme.body[range.start..range.end])
            .zip(body_arrangement.0.iter())
            .map(|(multiple_choice, choice)| {
                multiple_choice
                    .split(|l| matches!(l, WithSpan(BodyLexeme::ChoiceDelim, _, _)))
                    .nth(*choice)
                    .expect("Unreachable: Head choice index exceeded the current section")
            })
            .for_each(|str_sections| {
                str_sections.iter().for_each(|lexeme| match lexeme {
                    WithSpan(BodyLexeme::Section, _, _) => self.scripts.push(lexeme.as_str()),
                    _ => unreachable!("Error on {:?}. Should be processed", lexeme),
                });
                //println!("{:?}", str_sections)
            });

        self.hotkeys.push((
            head_begin..self.chords.len(),
            body_begin..self.scripts.len(),
        ));
        //println!("{:?}", self.hotkeys);
        Ok(())
    }
}

/****************************************************************************
 * Helpers
 ****************************************************************************/
enum Either<'a, 'filestr> {
    H(&'a WithSpan<'filestr, HeadLexeme>),
    B(&'a WithSpan<'filestr, BodyLexeme>),
}

#[derive(Debug)]
struct Metadata {
    choice_count: usize,
    partition_count: usize, // A section is
    partition_width: usize, // The number of chords/sections
    space: SpaceCalc,
}

impl Metadata {
    fn new() -> Self {
        Self {
            choice_count: 0,
            partition_count: 0,
            partition_width: 0,
            space: SpaceCalc::new(),
        }
    }
}

// Full run of `parse()` on head and generator
// This entails running `parse()` twice for head and body
// The source code is takes up a lot of space
fn parse_lexeme(
    lexeme: &Lexeme,
    head_generator: &mut Option<&mut Permutations>,
    body_generator: &mut Option<&mut Permutations>,
) -> Result<(Metadata, Metadata), MarkupError> {
    let mut head_data = Metadata::new();
    lexeme
        .head
        .iter()
        .enumerate()
        .try_for_each(|(i, head_lexeme)| {
            let a = parse(
                &mut State::Loop,
                &mut head_data,
                i,
                head_generator,
                Either::H(head_lexeme),
            );
            //            println!("Head {:?}", head_data.space);
            a
        })?;
    parse(
        &mut State::End,
        &mut head_data,
        lexeme.head.len(),
        head_generator,
        Either::H(&WithSpan(HeadLexeme::Blank, "", 0..0)),
    )?;

    let mut body_data = Metadata::new();
    lexeme
        .body
        .iter()
        .enumerate()
        .try_for_each(|(i, body_lexeme)| {
            let a = parse(
                &mut State::Loop,
                &mut body_data,
                i,
                body_generator,
                Either::B(body_lexeme),
            );
            //println!("Loop {:?}", body_data.space);
            a
        })?;
    parse(
        &mut State::End,
        &mut body_data,
        lexeme.body.len(),
        body_generator,
        Either::B(&WithSpan(BodyLexeme::Section, "", 0..0)),
    )?;
    //println!("head done {:?}", head_data.space);
    //println!("body done {:?}", body_data.space);
    Ok((head_data, body_data))
}

impl Chord {
    fn add(&mut self, lexeme: &WithSpan<HeadLexeme>) -> Result<(), MarkupError> {
        match lexeme {
            WithSpan(HeadLexeme::Key(k), _, _) => {
                if self.key >= KEYCODES.len() {
                    self.key = *k;
                    Ok(())
                } else {
                    Err(lexeme.to_error("Only specify one key per chord"))
                }
            }
            WithSpan(HeadLexeme::Mod(m), _, _) => {
                let flag = 1 << *m;
                if self.modifiers & flag == 0 {
                    self.modifiers |= flag;
                    Ok(())
                } else {
                    Err(lexeme.to_error("Duplicate modifier"))
                }
            }
            // This is actually
            _ => unreachable!("Only for HeadLexeme::Key() and HeadLexeme::Mod()"),
        }
    }
}

/****************************************************************************
 * Mathematics
 ****************************************************************************/

#[derive(Debug)]
struct SpaceCalc {
    permutations: usize,
    items: usize,
}

impl SpaceCalc {
    fn new() -> Self {
        Self {
            permutations: 1,
            items: 0,
        }
    }

    // Inductively calculate the size requirements
    // Formula: GroupCount[i] * TotalSpace[i-1] + Permutations[i-1] * GroupSpace[i]
    // 'group_size' is number of choices
    // 'group_space_total' is the number of items across all choices
    fn calc_space(&mut self, group_size: usize, group_space_total: usize) -> usize {
        self.items = group_size * self.items + self.permutations * group_space_total;
        self.permutations *= group_size;
        self.items
    }
}

// This is memory designed to be shared across all lexemes
// And the
// Radices will always be a minimum of 1
#[derive(Debug)]
struct Permutations {
    // All these lengths should be the same (but weight is + 1)
    // We will be using '.radices' for the length
    cursor: Cursor,                // Cursor into head/body lexemes
    radices: Vec<usize>,           // (digits) The number of choices per partition
    weights: Vec<usize>,           // Weights
    arrangement: Vec<usize>,       // A specific permutation
    partitions: Vec<Range<usize>>, // Range into the head/body lexeme array
}

//impl<'a> Permutations<'a> {
impl Permutations {
    fn new(partition_count_max: usize) -> Self {
        let mut weights = Vec::with_capacity(partition_count_max + 1);
        weights.push(1);
        Self {
            cursor: Cursor(0),
            radices: Vec::with_capacity(partition_count_max),
            weights,
            arrangement: vec![0; partition_count_max],
            partitions: vec![0..0; partition_count_max],
        }
    }
    fn reset(&mut self) {
        self.radices.clear(); // Set length to 0
        self.partitions.clear();
        self.weights.truncate(1); // Set length to 1
        self.cursor.0 = 0;
    }

    fn push_digit_weight(&mut self, partition_width: usize, rindex: usize) {
        let last_weight = self.weights.last().copied().unwrap_or(1);
        self.radices.push(partition_width);
        self.weights.push(last_weight * partition_width);
        self.partitions.push(self.cursor.move_to(rindex));
    }

    fn generate_arrangement(&mut self, index: usize) -> (&[usize], &[Range<usize>]) {
        debug_assert_eq!(self.radices.capacity() + 1, self.weights.capacity());
        debug_assert_eq!(self.arrangement.capacity(), self.radices.capacity());
        self.radices.iter()
            .zip(self.weights.iter())
            .zip(self.arrangement.iter_mut())
            .for_each(|((radix, weight), digit)| *digit = (index / weight) % radix);
        let len = self.radices.len();
        (&self.arrangement[0..len], &self.partitions[0..len])
    }

    fn permutations(&self) -> usize {
        self.radices.iter().product()
    }
}

//#[test]
fn permutation_generator() {
    let mut gen = Permutations::new(5);
    gen.push_digit_weight(1, 0);
    gen.push_digit_weight(5, 0);
    gen.push_digit_weight(3, 0);
    let permutations = gen.permutations();
    let mut arrangements = Vec::with_capacity(permutations);
    for i in 0..permutations {
        arrangements.push(gen.generate_arrangement(i).0.to_vec());
    }
    arrangements.sort_unstable();
    arrangements.dedup();
    assert_eq!(permutations, arrangements.len());
}
