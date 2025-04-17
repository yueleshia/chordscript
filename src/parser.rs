use std::cmp;
use std::mem;
use std::ops::Range;

use crate::constants::KEYCODES;
use crate::define_syntax;
use crate::errors;
use crate::lexer::{BodyType, HeadType, Lexeme, LexemeOwner};
use crate::reporter::MarkupError;
use crate::structs::{Chord, Cursor, Shortcut, WithSpan};

// run: cargo test -- --nocapture
//run: cargo run list-debug -c $HOME/interim/hk/config.txt

/****************************************************************************
 * Structs
 ****************************************************************************/

#[derive(Debug)]
pub struct ShortcutOwner<'filestr> {
    // second Range<usize> will always be 0..0
    placeholders: Vec<(Range<usize>, Range<usize>)>,
    shortcuts: Vec<(Range<usize>, Range<usize>)>,
    chords: Vec<WithSpan<'filestr, Chord>>,
    scripts: Vec<WithSpan<'filestr, ()>>,
}

type ShortcutView<'owner, 'filestr> = Vec<Shortcut<'owner, 'filestr>>;

// Choose this API to meaningfully separate a sorted and original-order view
// Also keeps the fields in 'ShortcutOwner' private
impl<'filestr> ShortcutOwner<'filestr> {
    pub fn make_owned_sorted_view<'owner>(&'owner self) -> ShortcutView<'owner, 'filestr> {
        let mut view = Vec::with_capacity(self.shortcuts.len());
        view.extend(self.to_iter());

        // Prefer stable sort to keep ordering for duplicate line errors
        // @VOLATILE search method
        view.sort_by(|a, b| a.hotkey.cmp(b.hotkey));
        view
    }

    pub fn to_iter<'owner>(&'owner self) -> ShortcutIter<'owner, 'filestr> {
        ShortcutIter {
            iter: self.shortcuts.iter(),
            owner: self,
        }
    }

    pub fn to_placeholder_iter<'owner>(&'owner self) -> ShortcutIter<'owner, 'filestr> {
        ShortcutIter {
            iter: self.placeholders.iter(),
            owner: self,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ShortcutIter<'owner, 'filestr> {
    iter: std::slice::Iter<'owner, (Range<usize>, Range<usize>)>,
    owner: &'owner ShortcutOwner<'filestr>,
}

impl<'owner, 'filestr> Iterator for ShortcutIter<'owner, 'filestr> {
    type Item = Shortcut<'owner, 'filestr>;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(head, body)| Shortcut {
            hotkey: &self.owner.chords[head.start..head.end],
            command: &self.owner.scripts[body.start..body.end],
        })
    }
}

/****************************************************************************
 * Macros
 ****************************************************************************/
// Outputs a '(usize, <$source as Iterator>::Item)' with the usize is the index
// of the found item
macro_rules! find_prev_from {
    ($source:ident, $i:ident, $search_for:pat $(| $search_for2:pat)* ) => {
        $source.iter().enumerate().take($i).rfind(|(_, lexeme)|
            matches!(lexeme.data, $search_for $( | $search_for2 )*)
        ).unwrap()
    };
}
macro_rules! find_next_from {
    ($source:ident, $i:ident, $search_for:pat $(| $search_for2:pat)* ) => {
        $source.iter().enumerate().skip($i).find(|(_, lexeme)|
            matches!(lexeme.data, $search_for $( | $search_for2 )*)
        ).unwrap()
    };
}

/****************************************************************************
 * Syntax
 ****************************************************************************/
pub fn process<'filestr>(
    lexeme_stream: &'filestr LexemeOwner<'filestr>,
) -> Result<ShortcutOwner<'filestr>, MarkupError> {
    let (
        placeholder_capacity,
        rendered_head_capacity,
        rendered_body_capacity,
        hotkey_capacity,
        partition_max,
    ) = {
        let mut placeholder_capacity = 0;
        let mut rendered_head_capacity = 0;
        let mut rendered_body_capacity = 0;
        let mut hotkey_capacity = 0;
        let mut partition_max = 0;
        for lexeme in lexeme_stream.to_iter() {
            let (head_data, body_data) = parse_lexeme(&lexeme, &mut None, &mut None)?;

            match lexeme.head[0].data {
                HeadType::Hotkey => hotkey_capacity += head_data.space.permutations,
                HeadType::Placeholder => placeholder_capacity += head_data.space.permutations,
                _ => unreachable!(),
            }
            rendered_head_capacity += head_data.space.items;
            rendered_body_capacity += body_data.space.items;

            partition_max = cmp::max(partition_max, head_data.partition_count);
            partition_max = cmp::max(partition_max, body_data.partition_count);
        }
        (
            placeholder_capacity,
            rendered_head_capacity,
            rendered_body_capacity,
            hotkey_capacity,
            partition_max,
        )
    };

    let parsemes = {
        //let (arrangement, permutations) = &mut Permutations::new(partition_max);
        //permutations.link_to(arrangement);
        // @VOLATILE depends on when reset is called
        let head_generator = &mut Permutations::new(partition_max);
        let body_generator = &mut Permutations::new(partition_max);
        let mut owner = ShortcutOwner {
            placeholders: Vec::with_capacity(placeholder_capacity),
            shortcuts: Vec::with_capacity(hotkey_capacity),
            chords: Vec::with_capacity(rendered_head_capacity),
            scripts: Vec::with_capacity(rendered_body_capacity),
        };
        for lexeme in lexeme_stream.to_iter() {
            debug_assert!(matches!(
                lexeme.head[0].data,
                HeadType::Hotkey | HeadType::Placeholder
            ));
            // @VOLATILE depends on when reset is called
            head_generator.reset(1); // skip HeadType::Hotkey and HeadType::Placeholder
            body_generator.reset(0);

            // Head
            let (head_data, _) = parse_lexeme(
                &lexeme,
                &mut Some(head_generator),
                &mut Some(body_generator),
            )?;

            // Build owner
            let arrangement_count = head_data.space.permutations;
            for i in 0..arrangement_count {
                let head = head_generator.generate_arrangement(i);
                let body = body_generator.generate_arrangement(i);
                owner.push_arrangement(&lexeme, head, body)?;
            }
        }

        debug_assert_eq!(placeholder_capacity, owner.placeholders.len());
        debug_assert_eq!(hotkey_capacity, owner.shortcuts.len());
        debug_assert_eq!(rendered_head_capacity, owner.chords.len());
        // @TODO debug this
        //debug_assert_eq!(rendered_body_capacity, owner.scripts.len());
        owner
    };

    verify_no_overlap(&parsemes)?;

    Ok(parsemes)
}

// Verify that all hotkeys are accessible (and no duplicates)
// e.g. 'super + a' and 'super + a; super + b' cannot be used at the same time
fn verify_no_overlap(parsemes: &ShortcutOwner) -> Result<(), MarkupError> {
    // @TODO add test for making sure placeholders will conflict with shortcuts
    let sorted_shortcuts = {
        let ShortcutOwner {
            shortcuts,
            placeholders,
            ..
        } = parsemes;
        let mut all = Vec::with_capacity(shortcuts.len() + placeholders.len());
        all.extend(parsemes.to_iter().map(|x| (HeadType::Hotkey, x)));
        all.extend(
            parsemes
                .to_placeholder_iter()
                .map(|x| (HeadType::Placeholder, x)),
        );
        // @VOLATILE search method
        all.sort_by(|a, b| a.1.hotkey.cmp(b.1.hotkey));
        all
    };
    let mut iter = sorted_shortcuts
        .iter()
        .map(|(ty, shortcut)| (ty, shortcut.hotkey));

    if let Some(first) = iter.next() {
        iter.try_fold(
            first,
            |(prev_type, prev_hotkey), (curr_type, curr_hotkey)| {
                let prev_len = prev_hotkey.len();
                let curr_len = curr_hotkey.len();
                if curr_len >= prev_len && &curr_hotkey[0..prev_len] == prev_hotkey {
                    // @TODO '|q; c|' and '|q|' does not have the correct span
                    let last = &curr_hotkey[prev_len - 1];
                    let has_placeholder = matches!(prev_type, HeadType::Placeholder)
                        || matches!(prev_type, HeadType::Placeholder);
                    // @TODO set span to non_place holder
                    let message = match (has_placeholder, curr_len == prev_len) {
                        (false, true) => errors::HOTKEY_DUPLICATE,
                        (false, false) => errors::HOTKEY_UNREACHABLE,
                        (true, true) => errors::PLACEHOLDER_DUPLICATE,
                        (true, false) => errors::PLACEHOLDER_UNREACHABLE,
                    };
                    // @TODO include two source code underlines
                    return Err(MarkupError::from_range(
                        last.context,
                        curr_hotkey[0].span_to_as_range(last),
                        message.to_string(),
                    ));
                }

                Ok((curr_type, curr_hotkey))
            },
        )?;
    }
    Ok(())
}

define_syntax! {
    parse_syntax | state: State
        ! data: &mut Metadata, index: usize, is_push: &mut Option<&mut Permutations>,
        (lexeme: Either)
    | -> (),
    Loop {
        Either::H(HeadType::ChordDelim) => data.partition_width += 1;
        Either::B(BodyType::Section) => data.partition_width += 1;

        Either::H(HeadType::ChoiceBegin) | Either::B(BodyType::ChoiceBegin) => {
            if let Some(generator) = is_push {
                generator.push_digit_weight(1, index);
                // Skip HeadType::ChoiceBegin
                generator.cursor.move_to(index + 1);
            }
            data.space.calc_space(1, data.partition_width);

            data.choice_count = 0;
            data.partition_width = 0;
            data.partition_count += 1;
        };

        Either::H(HeadType::ChoiceDelim) | Either::B(BodyType::ChoiceDelim) =>
            data.choice_count += 1;

        Either::H(HeadType::ChoiceClose) | Either::B(BodyType::ChoiceClose)  => {
            if let Some(generator) = is_push {
                generator.push_digit_weight(data.choice_count + 1, index);
                // Skip HeadType::ChoiceClose
                generator.cursor.move_to(index + 1);
            }
            data.space.calc_space(data.choice_count + 1, data.partition_width);
            //data.choice_count = 0; // Only needed if we reset
            data.partition_width = 0;
            data.partition_count += 1;
        };
        _ => {};
    }

    End {
        l => {
            // @TODO Investigate why head is +1 but body is not
            // Because head always adds HeadType::Blank, but body does not
            // add a BodyType::Section with empty span?
            match l {
                Either::H(_) => data.space.calc_space(1, data.partition_width + 1),
                Either::B(_) => data.space.calc_space(1, data.partition_width),
            };
            data.partition_count += 1;
            // NOTE: index for State::End is the rindex (+ 1 of normal)
            if let Some(generator) = is_push {
                // Include till end
                generator.push_digit_weight(1, index);
                debug_assert_eq!(data.space.permutations, generator.permutations());
            }
        };
    }
}

/****************************************************************************
 * Helpers
 ****************************************************************************/
enum Either<'a> {
    H(&'a HeadType),
    B(&'a BodyType),
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
    Lexeme { head, body }: &Lexeme,
    head_generator: &mut Option<&mut Permutations>,
    body_generator: &mut Option<&mut Permutations>,
) -> Result<(Metadata, Metadata), MarkupError> {
    let mut head_data = Metadata::new();
    head.iter().enumerate().try_for_each(|(i, head_lexeme)| {
        parse_syntax(
            &mut State::Loop,
            &mut head_data,
            i,
            head_generator,
            Either::H(&head_lexeme.data),
        )
    })?;
    parse_syntax(
        &mut State::End,
        &mut head_data,
        head.len(),
        head_generator,
        Either::H(&HeadType::Blank),
    )?;

    let mut body_data = Metadata::new();
    body.iter().enumerate().try_for_each(|(i, body_lexeme)| {
        parse_syntax(
            &mut State::Loop,
            &mut body_data,
            i,
            body_generator,
            Either::B(&body_lexeme.data),
        )?;

        // 'space.permutations' only increases on BodyType::ChoiceClose
        if body_data.space.permutations > head_data.space.permutations {
            return Err(choice_count_error(body, i));
        }
        //println!("{:?} {:?}", body_lexeme.as_str(), body_data.space);
        Ok(())
    })?;

    parse_syntax(
        &mut State::End,
        &mut body_data,
        body.len(),
        body_generator,
        Either::B(&BodyType::Section),
    )?;
    //println!("done {:?}\n", body_data.space);
    Ok((head_data, body_data))
}

// @TODO pair command for push_arrangement
fn map_arrangement_to_hotkey_sections<'parsemes: 'b, 'filestr, 'b, T>(
    list: &'parsemes [WithSpan<'filestr, T>],
    arrangement: &'b [usize],
    partitioning: &'b [Range<usize>],
    section_delim: mem::Discriminant<T>,
) -> impl Iterator<Item = std::slice::Iter<'b, WithSpan<'filestr, T>>>
//where 'parsemes: 'b
{
    arrangement
        .iter()
        .zip(
            partitioning
                .iter()
                .map(move |range| &list[range.start..range.end]),
        )
        .map(move |(choice, section)| {
            section
                //.split(|l| &l.data == &section_delim)
                .split(|l| mem::discriminant(&l.data) == section_delim)
                //.split(|l| matches!(&l.data, section_delim))
                .nth(*choice)
                .unwrap() // Syntax guarantees 'choice'-th index
                .iter()
        })
}
impl<'filestr> ShortcutOwner<'filestr> {
    fn push_arrangement(
        &mut self,
        Lexeme { head, body }: &Lexeme<'_, 'filestr>,
        (head_arrangement, head_partitioning): (&[usize], &[Range<usize>]),
        (body_arrangement, body_partitioning): (&[usize], &[Range<usize>]),
    ) -> Result<(), MarkupError> {
        //self.head_arrange
        debug_assert_eq!(head_arrangement.len(), head_partitioning.len());
        debug_assert_eq!(body_arrangement.len(), body_partitioning.len());

        let (head_begin, body_begin) = (self.chords.len(), self.scripts.len());
        let shortcut_type = &head[0]; // HeadType::Hotkey | HeadType::Placeholder
        let mut cursor = Cursor(head[1].range.start);
        //let a = map_arrangement_to_hotkey_sections(head, head_arrangement, head_partitioning, HeadType::ChoiceDelim);
        let last_chord = map_arrangement_to_hotkey_sections(
            head,
            head_arrangement,
            head_partitioning,
            mem::discriminant(&HeadType::ChoiceDelim),
        )
        .try_fold(Chord::new(), |o_chord, mut hotkey_sections| {
            hotkey_sections.try_fold(o_chord, |chord, keyspan| match keyspan.data {
                HeadType::ChordDelim => {
                    self.chords.push(WithSpan {
                        data: chord,
                        context: keyspan.context,
                        range: cursor.move_to(keyspan.range.end),
                    });
                    Ok(Chord::new())
                }
                HeadType::Mod(_) => chord.add(keyspan),
                HeadType::Key(_) => chord.add(keyspan),
                HeadType::Blank => Ok(chord),
                //HeadType::Hotkey | HeadType::Placeholder => Ok(chord),
                _ => unreachable!("{}", keyspan.to_error(errors::PANIC_NON_KEY)),
            })
        })?;

        // If no hotkeys were built error else push
        if last_chord == Chord::new() {
            let last = &head.last().unwrap().range;
            return Err(MarkupError::from_range(
                head[1].context,
                if head_begin == self.chords.len() {
                    const BAR: usize = "|".len(); // Include bars around head
                    head[1].range.start - BAR..last.end + BAR
                } else {
                    let len = head.len();
                    let (index, _) = find_prev_from!(head, len, HeadType::ChordDelim);
                    head[index].range.start..last.end
                },
                errors::EMPTY_HOTKEY.to_string(),
            ));
        } else {
            let last_span = &head.last().unwrap();
            self.chords.push(WithSpan {
                data: last_chord,
                context: last_span.context,
                range: cursor.move_to(last_span.range.end),
            });
        }

        map_arrangement_to_hotkey_sections(
            body,
            body_arrangement,
            body_partitioning,
            mem::discriminant(&BodyType::ChoiceDelim),
        )
        .for_each(|str_sections| {
            str_sections.for_each(|lexeme| match lexeme.data {
                BodyType::Section => self.scripts.push(lexeme.map_to(())),
                _ => unreachable!("{}", lexeme.to_error(errors::PANIC_CHOICE_NON_SECTION)),
            });
        });

        match shortcut_type.data {
            HeadType::Hotkey => &mut self.shortcuts,
            HeadType::Placeholder => &mut self.placeholders,
            _ => unreachable!(),
        }
        .push((
            head_begin..self.chords.len(),
            body_begin..self.scripts.len(),
        ));
        Ok(())
    }
}

// These debug_asserts are guarantees by the syntax
fn choice_count_error(body: &[WithSpan<BodyType>], i: usize) -> MarkupError {
    debug_assert!(matches!(
        body[i].data,
        BodyType::ChoiceDelim | BodyType::ChoiceClose
    ));

    // BodyType::ChoiceDelim guaranteed to exist because we always start with
    // one permutation (i.e. choice of one is never an error)
    let (prev_delim, _) = find_prev_from!(body, i, BodyType::ChoiceDelim);
    debug_assert!(matches!(
        body[prev_delim].data,
        BodyType::ChoiceDelim | BodyType::ChoiceBegin
    ));

    let (brackets, _) = find_next_from!(body, i, BodyType::ChoiceClose);
    debug_assert!(matches!(
        body[i].data,
        BodyType::ChoiceDelim | BodyType::ChoiceClose
    ));

    MarkupError::from_range(
        body[i].context,
        WithSpan::span_to_as_range(&body[prev_delim + 1], &body[brackets - 1]),
        errors::TOO_MUCH_BODY.to_string(),
    )
}

impl Chord {
    fn add(mut self, lexeme: &WithSpan<HeadType>) -> Result<Self, MarkupError> {
        match lexeme.data {
            HeadType::Key(k) => {
                if self.key >= KEYCODES.len() {
                    self.key = k;
                    Ok(self)
                } else {
                    Err(lexeme.to_error("Only specify one key per chord"))
                }
            }
            HeadType::Mod(m) => {
                let flag = 1 << m;
                if self.modifiers & flag == 0 {
                    self.modifiers |= flag;
                    Ok(self)
                } else {
                    Err(lexeme.to_error("Duplicate modifier"))
                }
            }
            // This is actually
            _ => unreachable!("Only for HeadType::Key() and HeadType::Mod()"),
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
            // @VOLATILE we call reset directly after so default 0 is never used
            cursor: Cursor(0),
            radices: Vec::with_capacity(partition_count_max),
            weights,
            arrangement: vec![0; partition_count_max],
            partitions: vec![0..0; partition_count_max],
        }
    }
    fn reset(&mut self, index: usize) {
        self.radices.clear(); // Set length to 0
        self.partitions.clear();
        self.weights.truncate(1); // Set length to 1
        self.cursor.0 = index;
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
        self.radices
            .iter()
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

#[test]
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
