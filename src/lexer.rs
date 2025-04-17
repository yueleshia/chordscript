//run: cargo test -- --nocapture

use crate::constants::{KEYCODES, KEYSTR_UTF8_MAX_LEN, MODIFIERS, SEPARATOR};
use crate::define_syntax;
use crate::errors;
use crate::reporter::MarkupError;
use crate::structs::{Cursor, WithSpan};
use std::ops::Range;

/****************************************************************************
 * Token definitions
 ****************************************************************************/
#[derive(Debug)]
pub struct Lexeme<'owner, 'filestr> {
    pub head: &'owner [WithSpan<'filestr, HeadLexeme>],
    pub body: &'owner [WithSpan<'filestr, BodyLexeme>],
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
pub enum BodyLexeme {
    Section,
    ChoiceBegin,
    ChoiceDelim,
    ChoiceClose,
}

#[derive(Debug)]
pub struct LexemeOwner<'filestr> {
    entries: Vec<(Range<usize>, Range<usize>)>,
    head: Vec<WithSpan<'filestr, HeadLexeme>>,
    body: Vec<WithSpan<'filestr, BodyLexeme>>,
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
        data: &Metadata<'filestr>,
        keyrange: Range<usize>,
    ) -> Result<(), MarkupError> {
        let keystr = &data.source[keyrange.start..keyrange.end];
        if keyrange.is_empty() {
            self.head.push(data.head(HeadLexeme::Blank, keyrange));
        } else if let Some(i) = MODIFIERS.iter().position(|x| *x == keystr) {
            self.head.push(data.head(HeadLexeme::Mod(i), keyrange));
        } else if let Some(i) = KEYCODES.iter().position(|x| *x == keystr) {
            self.head.push(data.head(HeadLexeme::Key(i), keyrange));
        } else {
            data.report(keystr, errors::HEAD_INVALID_KEY)?;
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
        let data = &mut Metadata::new(filestr);
        let state = &mut State::Head;
        while let Some(item) = data.next() {
            lex_syntax(state, data, None, item)?;
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
            data.entry_capacity + 1,
            data.head_capacity,
            data.body_capacity + 1,
        )
    };
    //println!("{:?}", capacity);

    // Lex into lexemes
    let lexemes = {
        let mut owner = LexemeOwner::new(capacity);
        let (head_cursor, body_cursor) = (&mut Cursor(0), &mut Cursor(0));
        let state = &mut State::Head;
        let data = &mut Metadata::new(filestr);
        while let Some(item) = data.next() {
            //println!("{} {:?} {:?}",
            //    data.rindex,
            //    item.0,
            //    item.2.chars().take(20).collect::<String>()
            //);
            lex_syntax(state, data, Some((&mut owner, head_cursor, body_cursor)), item)?;
        }

        let last_body = data.cursor.move_to(data.source.len());
        owner.body.push(data.body(BodyLexeme::Section, last_body));
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
        ! data: &mut Metadata<'a>, is_push: Option<(&mut LexemeOwner<'a>, &mut Cursor, &mut Cursor)>,
        (lexeme: <Metadata as Iterator>::Item)
    | -> (),

    Head {
        (',', i, _) => data.report(
            &data.source[i..i + ','.len_utf8()],
            errors::HEAD_COMMA_OUTSIDE_BRACKETS
        )?;

        ('\\', i, _) => data.report(
            &data.source[i..i + '\\'.len_utf8()],
            errors::HEAD_NO_ESCAPING,
        )?;

        ('|', i, _) => {
            *state = State::Body;

            let till_close = data.cursor.move_to(i);
            data.cursor.move_to(i + '|'.len_utf8());
            // No eating separator while in State::Body

            if let Some((tokens, _, _)) = is_push {
                tokens.head_push_key(data, till_close)?;
            }
            data.head_capacity += 1;
            //Ok((0, 1, 0))
        };

        ('{', i, _) => {
            if let Some(('{', _, _)) = data.next() { // Second '{'
                *state = State::HeadBrackets;

                let till_bracket = data.cursor.move_to(i);
                data.eat_charlist(&SEPARATOR);
                let brackets = data.cursor.move_to(data.rindex);

                if let Some((tokens, _, _)) = is_push {
                    tokens.head_push_key(data, till_bracket)?;
                    tokens.head.push(data.head(HeadLexeme::ChoiceBegin, brackets));
                }
                data.head_capacity += 2;
                //Ok((0, 2, 0))
            } else {
                data.report(
                    &data.source[i + '{'.len_utf8()..i + "{{".len()],
                    errors::MISSING_LBRACKET,
                )?;
            }
        };

        (ch, i, _) if ch == ';' || SEPARATOR.contains(&ch) => {
            let till_punctuation = data.cursor.move_to(i);
            data.eat_charlist(&SEPARATOR);
            let delim = data.cursor.move_to(data.rindex);

            if let Some((tokens, _, _)) = is_push {
                tokens.head_push_key(data, till_punctuation)?;
                if ch == ';' {
                    tokens.head.push(data.head(HeadLexeme::ChordDelim, delim));
                }
            }
            match ch {
                ';' => data.head_capacity += 2,
                _ => data.head_capacity += 1,
                //';' => Ok((0, 2, 0)),
                //_ => Ok((0, 1, 0)),
            }
        };


        (_, i, _) if data.cursor.width(i) > KEYSTR_UTF8_MAX_LEN => {
            panic!("Panic at the disco")
        };
        _ => {};
    }

    HeadBrackets {
        ('|', i, _) => data.report(
            &data.source[i..i + '|'.len_utf8()],
            errors::HEAD_INVALID_CLOSE,
        )?;

        ('\\', i, _) => data.report(
            &data.source[i..i + '\\'.len_utf8()],
            errors::HEAD_NO_ESCAPING,
        )?;

        ('}', i, _) => {
            if let Some(('}', _, _)) = data.next() { // second '}'
                *state = State::Head;

                let till_bracket = data.cursor.move_to(i);
                data.eat_charlist(&SEPARATOR);
                let brackets = data.cursor.move_to(data.rindex);

                if let Some((tokens, _, _)) = is_push {
                    tokens.head_push_key(data, till_bracket)?;
                    tokens.head.push(data.head(HeadLexeme::ChoiceClose, brackets));
                }
                data.head_capacity += 2;
                //Ok((0, 2, 0))
            } else {
                data.report(
                    &data.source[i + '}'.len_utf8()..i + "}}".len()],
                    errors::MISSING_RBRACKET,
                )?;
            }
        };

        (ch, i, _) if ch == ';' || ch == ',' || SEPARATOR.contains(&ch) => {
            let till_punctuation = data.cursor.move_to(i);
            data.eat_charlist(&SEPARATOR);
            let punctuation = data.cursor.move_to(data.rindex);
            if let Some((tokens, _, _)) = is_push {
                tokens.head_push_key(data, till_punctuation)?;
                match ch {
                    ';' => tokens.head.push(data.head(HeadLexeme::ChordDelim, punctuation)),
                    ',' => tokens.head.push(data.head(HeadLexeme::ChoiceDelim, punctuation)),
                    _ => {}
                }
            }
            match ch {
                ';' | ',' => data.head_capacity += 2,
                _ => data.head_capacity += 1,
                //';' | ',' => Ok((0, 2, 0)),
                //_ => Ok((0, 1, 0)),
            }
        };

        (_, i, _) if data.cursor.width(i) > KEYSTR_UTF8_MAX_LEN => {
            panic!("Panic at the disco");
        };

        _ => {};
        //_ => Ok((0, 0, 0));
    }

    // No eating separator while in State::Body
    Body {
        ('\n', i, rest) if rest.starts_with("\n|") => {
            *state = State::Head;

            let include_newline = data.cursor.move_to(i);
            data.next(); // Skip '|'
            data.eat_charlist(&SEPARATOR); // Eat cause in State::Head
            data.cursor.move_to(i + "\n|".len());

            if let Some((tokens, head_cursor, body_cursor)) = is_push {
                tokens.body.push(data.body(BodyLexeme::Section, include_newline));
                tokens.push_entry(head_cursor, body_cursor);
            }
            data.entry_capacity += 1;
            data.body_capacity += 1;
            //Ok((1, 0, 1))
        };

        (_, i, rest) if rest.starts_with("{{{") => {
            *state = State::BodyLiteral;

            let till_bracket = data.cursor.move_to(i);
            data.next(); // Skip second '{'
            data.next(); // Skip third '{'
            data.cursor.move_to(data.rindex);

            if let Some((tokens, _, _)) = is_push {
                tokens.body.push(data.body(BodyLexeme::Section, till_bracket));
            }
            data.body_capacity += 1;
            //Ok((0, 0, 1))
        };

        ('{', i, _) if matches!(data.peek, Some('{')) => {
            *state = State::BodyBrackets;

            let till_bracket = data.cursor.move_to(i);
            data.next(); // Skip second '{'
            let brackets = data.cursor.move_to(data.rindex);

            if let Some((tokens, _, _)) = is_push {
                tokens.body.push(data.body(BodyLexeme::Section, till_bracket));
                tokens.body.push(data.body(BodyLexeme::ChoiceBegin, brackets));
            }
            data.body_capacity += 2;
            //Ok((0, 0, 2))
        };

        _ => {};
        //_ => Ok((0, 0, 0));
    }

    BodyLiteral {
        ('\\', i, _) => {
            let till_backslash = data.cursor.move_to(i);
            match data.next() {
                Some(('\n', _, _)) => {
                    data.cursor.move_to(data.rindex);

                    if let Some((tokens, _, _)) = is_push {
                        tokens.body.push(
                            data.body(BodyLexeme::Section, till_backslash)
                        );
                    }
                    data.body_capacity += 1;
                    //Ok((0, 0, 1))
                }
                Some((ch, j, _)) => {
                    data.cursor.move_to(data.rindex);

                    if let Some((tokens, _, _)) = is_push {
                        let escaped = j..j + ch.len_utf8();
                        tokens.body.push(
                            data.body(BodyLexeme::Section, till_backslash)
                        );
                        tokens.body.push(
                            data.body(BodyLexeme::Section, escaped)
                        );
                    }
                    data.body_capacity += 2;
                    //Ok((0, 0, 2))
                }
                // Let the final '\\' exist by itself
                None => {}
                //None => Ok((0, 0, 0)),
            }
        };

        (_, i, rest) if rest.starts_with("}}}") => {
            *state = State::Body;

            let till_bracket = data.cursor.move_to(i);
            data.next(); // Skip second '}'
            data.next(); // Skip third '}'
            data.cursor.move_to(data.rindex);

            if let Some((tokens, _, _)) = is_push {
                tokens.body.push(data.body(BodyLexeme::Section, till_bracket));
            }
            data.body_capacity += 1;
            //Ok((0, 0, 1))
        };

        _ => {};
        //_ => Ok((0, 0, 0));
    }

    BodyBrackets {
        ('}', i, _) if matches!(data.peek, Some('}')) => {
            *state = State::Body;

            let till_bracket = data.cursor.move_to(i);
            data.next(); // Skip second '}'
            let brackets = data.cursor.move_to(data.rindex);

            if let Some((tokens, _, _)) = is_push {
                tokens.body.push(data.body(BodyLexeme::Section, till_bracket));
                tokens.body.push(data.body(BodyLexeme::ChoiceClose, brackets));
            }
            data.body_capacity += 2;
            //Ok((0, 0, 2))
        };

        ('\\', i, _) => {
            let till_backslash = data.cursor.move_to(i);
            match data.next() {
                Some(('\n', _, _)) => {
                    data.cursor.move_to(data.rindex);

                    if let Some((tokens, _, _)) = is_push {
                        tokens.body.push(data.body(BodyLexeme::Section, till_backslash));
                        // Do not push '\n'
                    }
                    data.body_capacity += 1;
                    //Ok((0, 0, 1))
                }
                Some(_) => {
                    let escaped = data.cursor.move_to(data.rindex);

                    if let Some((tokens, _, _)) = is_push {
                        tokens.body.push(data.body(BodyLexeme::Section, till_backslash));
                        tokens.body.push(data.body(BodyLexeme::Section, escaped));
                    }
                    data.body_capacity += 2;
                    //Ok((0, 0, 2))
                }
                // Let the final '\\' exist by itself
                None => {}
                //None => Ok((0, 0, 0)),
            }
        };

        (',', i, _) => {
            let till_comma = data.cursor.move_to(i);
            let comma = data.cursor.move_to(data.rindex);
            if let Some((tokens, _, _)) = is_push {
                tokens.body.push(data.body(BodyLexeme::Section, till_comma));
                tokens.body.push(data.body(BodyLexeme::ChoiceDelim, comma));
            }
            data.body_capacity += 2;
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
struct Metadata<'filestr> {
    source: &'filestr str,
    walker: std::str::Chars<'filestr>,
    peek: Option<char>,
    rest: &'filestr str,
    rindex: usize,
    cursor: Cursor,

    entry_capacity: usize,
    head_capacity: usize,
    body_capacity: usize,
}
impl<'filestr> Metadata<'filestr> {
    fn new(source: &'filestr str) -> Self {
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


    fn head(&self, l: HeadLexeme, range: Range<usize>) -> WithSpan<'filestr, HeadLexeme> {
        WithSpan(l, self.source, range)
    }

    fn body(&self, l: BodyLexeme, range: Range<usize>) -> WithSpan<'filestr, BodyLexeme> {
        WithSpan(l, self.source, range)
    }

    fn report(&self, span: &'filestr str, message: &str) -> Result<(), MarkupError> {
        Err(MarkupError::new(&self.source, span, message.to_string()))
    }
}

impl<'filestr> Iterator for Metadata<'filestr> {
    type Item = (char, usize, &'filestr str);
    fn next(&mut self) -> Option<Self::Item> {
        let item = (self.peek?, self.rindex, self.rest);
        let len_utf8 = item.0.len_utf8();
        self.rest = self.walker.as_str();
        self.rindex += len_utf8;
        self.peek = self.walker.next();
        Some(item)
    }
}
