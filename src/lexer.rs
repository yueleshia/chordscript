//run: cargo test -- --nocapture

use crate::constants::{KEYCODES, KEYSTR_UTF8_MAX_LEN, MODIFIERS, SEPARATOR};
use crate::define_syntax;
use crate::errors;
use crate::reporter::MarkupError;
use crate::structs::Cursor;
use std::ops::Range;

/****************************************************************************
 * Token definitions
 ****************************************************************************/
#[derive(Debug)]
pub struct Lexeme<'owner, 'filestr> {
    pub head: &'owner [HeadLexeme],
    pub body: &'owner [BodyLexeme<'filestr>],
}

#[derive(Debug)]
pub enum HeadLexeme {
    Key(usize),
    Mod(usize),
    ChordDelim,
    Blank,
    ChoiceBegin,
    ChoiceDelim,
    ChoiceClose,
}

#[derive(Debug)]
pub enum BodyLexeme<'a> {
    Section(&'a str),
    ChoiceBegin,
    ChoiceDelim,
    ChoiceClose,
}

#[derive(Debug)]
pub struct LexemeOwner<'filestr> {
    entries: Vec<(Range<usize>, Range<usize>)>,
    head: Vec<HeadLexeme>,
    body: Vec<BodyLexeme<'filestr>>,
}

impl<'filestr> LexemeOwner<'filestr> {
    fn new(capacity: (usize, usize, usize)) -> Self {
        Self {
            entries: Vec::with_capacity(capacity.0),
            head: Vec::with_capacity(capacity.1),
            body: Vec::with_capacity(capacity.2),
        }
    }

    fn head_push_key(
        &mut self,
        metadata: &Metadata<'filestr>,
        keystr: &'filestr str,
    ) -> Result<(), MarkupError> {
        if keystr.is_empty() {
            self.head.push(HeadLexeme::Blank);
        } else if let Some(i) = MODIFIERS.iter().position(|x| *x == keystr) {
            self.head.push(HeadLexeme::Mod(i));
        } else if let Some(i) = KEYCODES.iter().position(|x| *x == keystr) {
            self.head.push(HeadLexeme::Key(i));
        } else {
            metadata.report(keystr, errors::HEAD_INVALID_KEY)?;
        }
        Ok(())
    }

    fn push_entry(&mut self, head_cursor: &mut Cursor, body_cursor: &mut Cursor) {
        self.entries.push((
            head_cursor.move_to(self.head.len()),
            body_cursor.move_to(self.body.len()),
        ));
    }

    pub fn to_iter(&self) -> impl Iterator<Item = Lexeme> {
        self.entries
            .iter()
            .map(move |(head_range, body_range)| Lexeme {
                head: &self.head[head_range.start..head_range.end],
                body: &self.body[body_range.start..body_range.end],
            })
    }
}

/****************************************************************************
 * Syntax
 ****************************************************************************/
// @TODO: Add test for Byte-Order Mark (BOM) ?
pub fn process(filestr: &str) -> Result<LexemeOwner, MarkupError> {
    // Skip until first '|' at beginning of line
    let filestr = {
        let mut walker = filestr.chars();
        let mut prev_ch = '\n';
        loop {
            match walker.next() {
                Some('|') if prev_ch == '\n' => break,
                Some(ch) => prev_ch = ch,
                None => return Ok(LexemeOwner::new((0, 0, 0))),
            }
        }
        walker.as_str()
    };

    // Calculate the memory needed for the Arrays
    let capacity = {
        let metadata = &mut Metadata::new(filestr);
        let state = &mut State::Head;
        while let Some(item) = metadata.next() {
            lex_syntax(state, metadata, None, item)?;
        }

        match state {
            State::Head => panic!("Head not closed"),
            State::HeadBrackets => panic!("Head bracket not closed"),
            State::Body => {} // Add one body and one entry
            State::BodyLiteral => panic!("Body literal not closed"),
            State::BodyBrackets => panic!("Body brackets not closed"),
        }
        // State::Body end not processed in loop
        (
            metadata.entry_capacity + 1,
            metadata.head_capacity,
            metadata.body_capacity + 1,
        )
    };
    //println!("{:?}", capacity);

    // Lex into lexemes
    let lexemes = {
        let mut owner = LexemeOwner::new(capacity);
        let (head_cursor, body_cursor) = (&mut Cursor(0), &mut Cursor(0));
        let state = &mut State::Head;
        let metadata = &mut Metadata::new(filestr);
        while let Some(item) = metadata.next() {
            //println!("{} {:?} {:?}",
            //    metadata.rindex,
            //    item.0,
            //    item.2.chars().take(20).collect::<String>()
            //);
            lex_syntax(state, metadata, Some((&mut owner, head_cursor, body_cursor)), item)?;
        }

        let last_body = &metadata.source[metadata.cursor.move_to(metadata.source.len())];
        owner.body.push(BodyLexeme::Section(last_body));
        owner.push_entry(head_cursor, body_cursor);
        owner
    };

    debug_assert_eq!(
        lexemes.head.len(),
        capacity.1,
        "Memory for head is incorrect"
    );
    debug_assert_eq!(
        lexemes.body.len(),
        capacity.2,
        "Memory for body is incorrect"
    );
    debug_assert_eq!(
        lexemes.entries.len(),
        capacity.0,
        "Memory for entires is incorrect"
    );

    Ok(lexemes)
}

// Basically one glorified match with these three variables as arguments
define_syntax! {
    lex_syntax | state: State
        ! metadata: &mut Metadata<'a>, is_push: Option<(&mut LexemeOwner<'a>, &mut Cursor, &mut Cursor)>,
        (lexeme: <Metadata as Iterator>::Item)
    | -> (),

    Head {
        (',', i, _) => metadata.report(
            &metadata.source[i..i + ','.len_utf8()],
            errors::HEAD_COMMA_OUTSIDE_BRACKETS
        )?;

        ('\\', i, _) => metadata.report(
            &metadata.source[i..i + '\\'.len_utf8()],
            errors::HEAD_NO_ESCAPING,
        )?;

        ('|', i, _) => {
            *state = State::Body;

            let till_close = &metadata.source[metadata.cursor.move_to(i)];
            metadata.cursor.move_to(i + '|'.len_utf8());
            // No eating separator while in State::Body

            if let Some((tokens, _, _)) = is_push {
                tokens.head_push_key(metadata, till_close)?;
            }
            metadata.head_capacity += 1;
            //Ok((0, 1, 0))
        };

        ('{', i, _) => {
            if let Some(('{', _, _)) = metadata.next() { // Second '{'
                *state = State::HeadBrackets;

                let till_bracket = &metadata.source[metadata.cursor.move_to(i)];
                metadata.eat_charlist(&SEPARATOR);
                metadata.cursor.move_to(metadata.rindex);

                if let Some((tokens, _, _)) = is_push {
                    tokens.head_push_key(metadata, till_bracket)?;
                    tokens.head.push(HeadLexeme::ChoiceBegin);
                }
                metadata.head_capacity += 2;
                //Ok((0, 2, 0))
            } else {
                metadata.report(
                    &metadata.source[i + '{'.len_utf8()..i + "{{".len()],
                    errors::MISSING_LBRACKET,
                )?;
            }
        };

        (ch, i, _) if ch == ';' || SEPARATOR.contains(&ch) => {
            let till_punctuation = &metadata.source[metadata.cursor.move_to(i)];
            metadata.eat_charlist(&SEPARATOR);
            metadata.cursor.move_to(metadata.rindex);

            if let Some((tokens, _, _)) = is_push {
                tokens.head_push_key(metadata, till_punctuation)?;
                if ch == ';' {
                    tokens.head.push(HeadLexeme::ChordDelim);
                }
            }
            match ch {
                ';' => metadata.head_capacity += 2,
                _ => metadata.head_capacity += 1,
                //';' => Ok((0, 2, 0)),
                //_ => Ok((0, 1, 0)),
            }
        };


        (_, i, _) if metadata.cursor.width(i) > KEYSTR_UTF8_MAX_LEN => {
            panic!("Panic at the disco")
        };
        _ => {};
    }

    HeadBrackets {
        ('|', i, _) => metadata.report(
            &metadata.source[i..i + '|'.len_utf8()],
            errors::HEAD_INVALID_CLOSE,
        )?;

        ('\\', i, _) => metadata.report(
            &metadata.source[i..i + '\\'.len_utf8()],
            errors::HEAD_NO_ESCAPING,
        )?;

        ('}', i, _) => {
            if let Some(('}', _, _)) = metadata.next() { // second '}'
                *state = State::Head;

                let till_bracket = &metadata.source[metadata.cursor.move_to(i)];
                metadata.eat_charlist(&SEPARATOR);
                metadata.cursor.move_to(metadata.rindex);

                if let Some((tokens, _, _)) = is_push {
                    tokens.head_push_key(metadata, till_bracket)?;
                    tokens.head.push(HeadLexeme::ChoiceClose);
                }
                metadata.head_capacity += 2;
                //Ok((0, 2, 0))
            } else {
                metadata.report(
                    &metadata.source[i + '}'.len_utf8()..i + "}}".len()],
                    errors::MISSING_RBRACKET,
                )?;
            }
        };

        (ch, i, _) if ch == ';' || ch == ',' || SEPARATOR.contains(&ch) => {
            let till_punctuation= &metadata.source[metadata.cursor.move_to(i)];
            metadata.eat_charlist(&SEPARATOR);
            metadata.cursor.move_to(metadata.rindex);
            if let Some((tokens, _, _)) = is_push {
                tokens.head_push_key(metadata, till_punctuation)?;
                match ch {
                    ';' => tokens.head.push(HeadLexeme::ChordDelim),
                    ',' => tokens.head.push(HeadLexeme::ChoiceDelim),
                    _ => {}
                }
            }
            match ch {
                ';' | ',' => metadata.head_capacity += 2,
                _ => metadata.head_capacity += 1,
                //';' | ',' => Ok((0, 2, 0)),
                //_ => Ok((0, 1, 0)),
            }
        };

        (_, i, _) if metadata.cursor.width(i) > KEYSTR_UTF8_MAX_LEN => {
            panic!("Panic at the disco");
        };

        _ => {};
        //_ => Ok((0, 0, 0));
    }

    // No eating separator while in State::Body
    Body {
        ('\n', i, rest) if rest.starts_with("\n|") => {
            *state = State::Head;

            let include_newline = &metadata.source[metadata.cursor.move_to(i)];
            metadata.next(); // Skip '|'
            metadata.eat_charlist(&SEPARATOR); // Eat cause in State::Head
            metadata.cursor.move_to(i + "\n|".len());

            if let Some((tokens, head_cursor, body_cursor)) = is_push {
                tokens.body.push(BodyLexeme::Section(include_newline));
                tokens.push_entry(head_cursor, body_cursor);
            }
            metadata.entry_capacity += 1;
            metadata.body_capacity += 1;
            //Ok((1, 0, 1))
        };

        (_, i, rest) if rest.starts_with("{{{") => {
            *state = State::BodyLiteral;

            let till_bracket = &metadata.source[metadata.cursor.move_to(i)];
            metadata.next(); // Skip second '{'
            metadata.next(); // Skip third '{'
            metadata.cursor.move_to(metadata.rindex);

            if let Some((tokens, _, _)) = is_push {
                tokens.body.push(BodyLexeme::Section(till_bracket));
            }
            metadata.body_capacity += 1;
            //Ok((0, 0, 1))
        };

        ('{', i, _) if matches!(metadata.peek, Some('{')) => {
            *state = State::BodyBrackets;

            let till_bracket = &metadata.source[metadata.cursor.move_to(i)];
            metadata.next(); // Skip second '{'
            metadata.cursor.move_to(metadata.rindex);

            if let Some((tokens, _, _)) = is_push {
                tokens.body.push(BodyLexeme::Section(till_bracket));
                tokens.body.push(BodyLexeme::ChoiceBegin);
            }
            metadata.body_capacity += 2;
            //Ok((0, 0, 2))
        };

        _ => {};
        //_ => Ok((0, 0, 0));
    }

    BodyLiteral {
        ('\\', i, _) => {
            let till_backslash = &metadata.source[metadata.cursor.move_to(i)];
            match metadata.next() {
                Some(('\n', _, _)) => {
                    metadata.cursor.move_to(metadata.rindex);

                    if let Some((tokens, _, _)) = is_push {
                        tokens.body.push(BodyLexeme::Section(till_backslash));
                    }
                    metadata.body_capacity += 1;
                    //Ok((0, 0, 1))
                }
                Some((ch, j, _)) => {
                    metadata.cursor.move_to(metadata.rindex);

                    if let Some((tokens, _, _)) = is_push {
                        let escaped = &metadata.source[j..j + ch.len_utf8()];
                        tokens.body.push(BodyLexeme::Section(till_backslash));
                        tokens.body.push(BodyLexeme::Section(escaped));
                    }
                    metadata.body_capacity += 2;
                    //Ok((0, 0, 2))
                }
                // Let the final '\\' exist by itself
                None => {}
                //None => Ok((0, 0, 0)),
            }
        };

        (_, i, rest) if rest.starts_with("}}}") => {
            *state = State::Body;

            let till_bracket = &metadata.source[metadata.cursor.move_to(i)];
            metadata.next(); // Skip second '}'
            metadata.next(); // Skip third '}'
            metadata.cursor.move_to(metadata.rindex);

            if let Some((tokens, _, _)) = is_push {
                tokens.body.push(BodyLexeme::Section(till_bracket));
            }
            metadata.body_capacity += 1;
            //Ok((0, 0, 1))
        };

        _ => {};
        //_ => Ok((0, 0, 0));
    }

    BodyBrackets {
        ('}', i, _) if matches!(metadata.peek, Some('}')) => {
            *state = State::Body;

            let till_bracket = &metadata.source[metadata.cursor.move_to(i)];
            metadata.next(); // Skip second '}'
            metadata.cursor.move_to(metadata.rindex);

            if let Some((tokens, _, _)) = is_push {
                tokens.body.push(BodyLexeme::Section(till_bracket));
                tokens.body.push(BodyLexeme::ChoiceClose);
            }
            metadata.body_capacity += 2;
            //Ok((0, 0, 2))
        };

        ('\\', i, _) => {
            let till_backslash = &metadata.source[metadata.cursor.move_to(i)];
            match metadata.next() {
                Some(('\n', _, _)) => {
                    metadata.cursor.move_to(metadata.rindex);

                    if let Some((tokens, _, _)) = is_push {
                        tokens.body.push(BodyLexeme::Section(till_backslash));
                        // Do not push '\n'
                    }
                    metadata.body_capacity += 1;
                    //Ok((0, 0, 1))
                }
                Some((ch, j, _)) => {
                    metadata.cursor.move_to(metadata.rindex);

                    if let Some((tokens, _, _)) = is_push {
                        let escaped = &metadata.source[j..j + ch.len_utf8()];
                        tokens.body.push(BodyLexeme::Section(till_backslash));
                        tokens.body.push(BodyLexeme::Section(escaped));
                    }
                    metadata.body_capacity += 2;
                    //Ok((0, 0, 2))
                }
                // Let the final '\\' exist by itself
                None => {}
                //None => Ok((0, 0, 0)),
            }
        };

        (',', i, _) => {
            let till_comma = &metadata.source[metadata.cursor.move_to(i)];
            metadata.cursor.move_to(metadata.rindex);
            if let Some((tokens, _, _)) = is_push {
                tokens.body.push(BodyLexeme::Section(till_comma));
                tokens.body.push(BodyLexeme::ChoiceDelim);
            }
            metadata.body_capacity += 2;
            //Ok((0, 0, 2))
        };

        //_ => Ok((0, 0, 0));
        _ => {};
    }
}

/****************************************************************************
 * Lexer Control-Flow Structs
 ****************************************************************************/

// This owns the data that represents our token streams
// Info specific to the 'to_push' = true branch of the lexer
//
struct Metadata<'a> {
    source: &'a str,
    walker: std::str::Chars<'a>,
    peek: Option<char>,
    rest: &'a str,
    rindex: usize,
    cursor: Cursor,

    entry_capacity: usize,
    head_capacity: usize,
    body_capacity: usize,
}
impl<'a> Metadata<'a> {
    fn new(source: &'a str) -> Self {
        let mut walker = source.chars();
        let peek = walker.next();
        Self {
            source,
            walker,
            rest: source,
            peek,
            rindex: 0,
            cursor: Cursor(0),

            entry_capacity: 0,
            head_capacity: 0,
            body_capacity: 0,
        }
    }
    fn eat_charlist(&mut self, list: &[char]) {
        while let Some(ch) = self.peek {
            if list.contains(&ch) {
                self.next();
            } else {
                break;
            }
        }
    }

    fn report(&self, span: &'a str, message: &str) -> Result<(), MarkupError> {
        Err(MarkupError::new(&self.source, span, message.to_string()))
    }
}

impl<'a> Iterator for Metadata<'a> {
    type Item = (char, usize, &'a str);
    fn next(&mut self) -> Option<Self::Item> {
        let item = (self.peek?, self.rindex, self.rest);
        let len_utf8 = item.0.len_utf8();
        self.rest = self.walker.as_str();
        self.rindex += len_utf8;
        self.peek = self.walker.next();
        Some(item)
    }
}
