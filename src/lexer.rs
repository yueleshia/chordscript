// run: cargo test -- --nocapture
// run: cargo run list-debug -c $HOME/interim/hk/config.txt
//run: cargo run keyspaces -c $HOME/interim/hk/config.txt

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
    pub head: &'owner [WithSpan<'filestr, HeadType>],
    pub body: &'owner [WithSpan<'filestr, BodyType>],
}

#[derive(Debug)]
pub enum HeadType {
    Hotkey,
    Placeholder,
    Comment,
    Key(usize),
    Mod(usize),
    ChordDelim,
    Blank,
    ChoiceBegin,
    ChoiceDelim,
    ChoiceClose,
}

#[derive(Clone, Debug)]
pub enum BodyType {
    Section,
    ChoiceBegin,
    ChoiceDelim,
    ChoiceClose,
}

#[derive(Debug)]
pub struct LexemeOwner<'filestr> {
    entries: Vec<(Range<usize>, Range<usize>)>,
    head: Vec<WithSpan<'filestr, HeadType>>,
    body: Vec<WithSpan<'filestr, BodyType>>,
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
            self.head.push(data.token(HeadType::Blank, keyrange));
        } else if let Some(i) = MODIFIERS.iter().position(|x| *x == keystr) {
            self.head.push(data.token(HeadType::Mod(i), keyrange));
        } else if let Some(i) = KEYCODES.iter().position(|x| *x == keystr) {
            self.head.push(data.token(HeadType::Key(i), keyrange));
        } else {
            return Err(data.report(keyrange, errors::HEAD_INVALID_KEY));
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
    // Skip until first '|' or '!' at beginning of line
    let markup = {
        let mut walker = filestr.chars();
        let mut prev_ch = '\n';
        let mut index = 0;
        loop {
            match walker.next() {
                Some('|') | Some('!') if prev_ch == '\n' => break,
                Some(ch) => {
                    prev_ch = ch;
                    index += ch.len_utf8();
                }
                None => return Ok(LexemeOwner::new((0, 0, 0))),
            }
        }
        &filestr[index..] // before the '|' or '!'
    };

    // Calculate the memory needed for the Arrays
    let capacity = {
        let data = &mut Metadata::new(filestr, markup);
        let state = &mut State::Initial;
        while let Some(item) = data.next() {
            lex_syntax(state, data, None, item)?;
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
        let state = &mut State::Initial;
        let data = &mut Metadata::new(filestr, markup);
        while let Some(item) = data.next() {
            lex_syntax(
                state,
                data,
                Some((&mut owner, head_cursor, body_cursor)),
                item,
            )?;
        }

        let end = data.source.len()..data.source.len() + 1;
        match state {
            State::Initial | State::Placeholder => Err(data.report(end, "Not sure what error yet")),
            State::Head => Err(data.report(end, errors::UNFINISHED_HEAD)),
            State::HeadBrackets => Err(data.report(end, errors::UNFINISHED_BRACKETS)),
            State::Body => {
                let last_body = data.cursor.move_to(data.source.len());
                owner.body.push(data.token(BodyType::Section, last_body));
                owner.push_entry(head_cursor, body_cursor);
                Ok(owner)
            }
            State::BodyLiteral => Err(data.report(end, errors::UNFINISHED_LITERAL)),
            State::BodyBrackets => Err(data.report(end, errors::UNFINISHED_BRACKETS)),
        }?
    };

    debug_assert_eq!(lexemes.entries.len(), capacity.0);
    debug_assert_eq!(lexemes.head.len(), capacity.1);
    debug_assert_eq!(lexemes.body.len(), capacity.2);
    Ok(lexemes)
}

// Basically one glorified match with these three variables as arguments
define_syntax! {
    lex_syntax | state: State
        ! data: &mut Metadata<'a>, is_push: Option<(&mut LexemeOwner<'a>, &mut Cursor, &mut Cursor)>,
        (lexeme: <Metadata as Iterator>::Item)
    | -> (),

    Initial {
        // Do not eat charlist so that HeadType::Blank has the correct span
        // i.e. "|  |" should trigger parser errors::EMTPY_HOTKEY
        //      and the span should be correct
        ('|', _, _) => {
            *state = State::Head;
            data.parent_state = State::Head;
            let bar = data.cursor.move_to(data.rindex); // Skip '|'

            if let Some((tokens, _, _)) = is_push {
                tokens.head.push(data.token(HeadType::Hotkey, bar));
            }
            data.head_capacity += 1;
        };
        ('!', _, _) => {
            *state = State::Placeholder;
            data.parent_state = State::Placeholder;
            let exclaim = data.cursor.move_to(data.rindex); // Skip '!'
            if let Some((tokens, _, _)) = is_push {
                tokens.head.push(data.token(HeadType::Placeholder, exclaim));
            }
            data.head_capacity += 1;
        };
        _ => unreachable!("Skipping before lex_syntax() is even called should stop this");
    }

    Head | Placeholder {
        (',', i, _) => return Err(data.report(
            i..i + ','.len_utf8(),
            errors::HEAD_COMMA_OUTSIDE_BRACKETS
        ));

        ('\\', i, _) => return Err(data.report(
            i..data.rindex,
            errors::HEAD_NO_ESCAPING,
        ));

        ('\n', i, rest) if rest.starts_with("\n#") => {
            let till_comment = data.cursor.move_to(i);
            data.peek_while(|c| c != '\n');
            data.cursor.move_to(data.rindex);

            if let Some((tokens, _, _)) = is_push {
                tokens.head_push_key(data, till_comment)?;
            }
            data.head_capacity += 1;
        };

        (ch @ '|', i, _) | (ch @ '!', i, _) => {
            match (&state, ch) {
                (State::Head, '|') => *state = State::Body,
                (State::Placeholder, '!') => *state = State::Body,

                // @TODO proper error message
                // @TODO two error
                _ => return Err(data.report(
                    i..data.rindex,
                    "TODO error, bar mismatch"
                )),
            }

            let till_close = data.cursor.move_to(i);
            data.cursor.move_to(data.rindex);
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
                    tokens.head.push(data.token(HeadType::ChoiceBegin, brackets));
                }
                data.head_capacity += 2;
                //Ok((0, 2, 0))
            } else {
                return Err(data.report(
                    i + '{'.len_utf8()..i + "{{".len(),
                    errors::MISSING_LBRACKET,
                ));
            }
        };

        (ch, i, _) if ch == ';' || SEPARATOR.contains(&ch) => {
            let till_punctuation = data.cursor.move_to(i);
            data.eat_charlist(&SEPARATOR);
            let delim = data.cursor.move_to(data.rindex);

            if let Some((tokens, _, _)) = is_push {
                tokens.head_push_key(data, till_punctuation)?;
                if ch == ';' {
                    tokens.head.push(data.token(HeadType::ChordDelim, delim));
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
            let till_now = data.cursor.move_to(i);
            return Err(data.report(till_now, errors::HEAD_INVALID_KEY));
        };
        _ => {};
    }

    HeadBrackets {
        ('\n', i, rest) if rest.starts_with("\n#") => {
            let till_comment = data.cursor.move_to(i);
            data.peek_while(|c| c != '\n');
            data.cursor.move_to(data.rindex);

            if let Some((tokens, _, _)) = is_push {
                tokens.head_push_key(data, till_comment)?;
            }
            data.head_capacity += 1;
        };

        ('|', i, _) => return Err(data.report(
            i..i + '|'.len_utf8(),
            errors::HEAD_INVALID_CLOSE,
        ));

        ('\\', i, _) => return Err(data.report(
            i..i + '\\'.len_utf8(),
            errors::HEAD_NO_ESCAPING,
        ));
        // @TODO error on '!'

        ('}', i, _) => {
            if let Some(('}', _, _)) = data.next() { // second '}'
                *state = data.parent_state.clone();

                let till_bracket = data.cursor.move_to(i);
                data.eat_charlist(&SEPARATOR);
                let brackets = data.cursor.move_to(data.rindex);

                if let Some((tokens, _, _)) = is_push {
                    tokens.head_push_key(data, till_bracket)?;
                    tokens.head.push(data.token(HeadType::ChoiceClose, brackets));
                }
                data.head_capacity += 2;
                //Ok((0, 2, 0))
            } else {
                return Err(data.report(
                    i + '}'.len_utf8()..i + "}}".len(),
                    errors::MISSING_RBRACKET,
                ));
            }
        };

        (ch, i, _) if ch == ';' || ch == ',' || SEPARATOR.contains(&ch) => {
            let till_punctuation = data.cursor.move_to(i);
            data.eat_charlist(&SEPARATOR);
            let punctuation = data.cursor.move_to(data.rindex);
            if let Some((tokens, _, _)) = is_push {
                tokens.head_push_key(data, till_punctuation)?;
                match ch {
                    ';' => tokens.head.push(data.token(HeadType::ChordDelim, punctuation)),
                    ',' => tokens.head.push(data.token(HeadType::ChoiceDelim, punctuation)),
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
            let till_now = data.cursor.move_to(i);
            return Err(data.report(till_now, errors::HEAD_INVALID_KEY));
        };

        _ => {};
        //_ => Ok((0, 0, 0));
    }

    // @TODO make HeadType::Placeholder skip its body in lexer step
    // No eating separator while in State::Body
    Body {
        ('\n', i, rest) if rest.starts_with("\n#") => {
            let till_comment = data.cursor.move_to(i);
            data.peek_while(|c| c != '\n');
            data.cursor.move_to(data.rindex);

            if let Some((tokens, _, _)) = is_push {
                tokens.body.push(data.token(BodyType::Section, till_comment));
            }
            data.body_capacity += 1;
        };

        ('\n', i, rest) if rest.starts_with("\n|") || rest.starts_with("\n!") => {
            *state = State::Initial;

            let include_newline = data.cursor.move_to(i);
            if let Some((tokens, head_cursor, body_cursor)) = is_push {
                tokens.body.push(data.token(BodyType::Section, include_newline));
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
                tokens.body.push(data.token(BodyType::Section, till_bracket));
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
                tokens.body.push(data.token(BodyType::Section, till_bracket));
                tokens.body.push(data.token(BodyType::ChoiceBegin, brackets));
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
                            data.token(BodyType::Section, till_backslash)
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
                            data.token(BodyType::Section, till_backslash)
                        );
                        tokens.body.push(
                            data.token(BodyType::Section, escaped)
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
                tokens.body.push(data.token(BodyType::Section, till_bracket));
            }
            data.body_capacity += 1;
            //Ok((0, 0, 1))
        };

        _ => {};
        //_ => Ok((0, 0, 0));
    }

    BodyBrackets {
        ('\n', i, rest) if rest.starts_with("\n#") => {
            let till_comment = data.cursor.move_to(i);
            data.peek_while(|c| c != '\n');
            data.cursor.move_to(data.rindex);

            if let Some((tokens, _, _)) = is_push {
                tokens.body.push(data.token(BodyType::Section, till_comment));
            }
            data.body_capacity += 1;
        };

        ('}', i, _) if matches!(data.peek, Some('}')) => {
            *state = State::Body;

            let till_bracket = data.cursor.move_to(i);
            data.next(); // Skip second '}'
            let brackets = data.cursor.move_to(data.rindex);

            if let Some((tokens, _, _)) = is_push {
                tokens.body.push(data.token(BodyType::Section, till_bracket));
                tokens.body.push(data.token(BodyType::ChoiceClose, brackets));
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
                        tokens.body.push(data.token(BodyType::Section, till_backslash));
                        // Do not push '\n'
                    }
                    data.body_capacity += 1;
                    //Ok((0, 0, 1))
                }
                Some(_) => {
                    let escaped = data.cursor.move_to(data.rindex);

                    if let Some((tokens, _, _)) = is_push {
                        tokens.body.push(data.token(BodyType::Section, till_backslash));
                        tokens.body.push(data.token(BodyType::Section, escaped));
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
                tokens.body.push(data.token(BodyType::Section, till_comma));
                tokens.body.push(data.token(BodyType::ChoiceDelim, comma));
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
    parent_state: State,
    original: &'filestr str, // File before initial skip, for error propagation
    source: &'filestr str,   // This is a substr of 'original'
    walker: std::str::Chars<'filestr>,

    // Not using source.chars().peekable() because still want 'walker.as_str()'
    // We need 'peek' for 'eat_charlist()' to not step too far
    peek: Option<char>,
    rest: &'filestr str, // source[i..] including current char
    rindex: usize,       // source[<Self as Iterator>::Item.1..rindex] is the current char
    cursor: Cursor,

    entry_capacity: usize,
    head_capacity: usize,
    body_capacity: usize,
}
impl<'filestr> Metadata<'filestr> {
    fn new(original: &'filestr str, source: &'filestr str) -> Self {
        let mut walker = source.chars();
        let peek = walker.next();
        Self {
            parent_state: State::Head,
            original,
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

    fn peek_while<F: Fn(char) -> bool>(&mut self, sentinel: F) -> Option<<Self as Iterator>::Item> {
        let mut current = None;
        while let Some(ch) = self.peek {
            if sentinel(ch) {
                current = self.next();
            } else {
                break;
            }
        }
        return current;
    }

    fn eat_charlist(&mut self, list: &[char]) {
        self.peek_while(|c| list.contains(&c));
    }

    fn token<T>(&self, token: T, range: Range<usize>) -> WithSpan<'filestr, T> {
        let offset = self.original.len() - self.source.len();
        WithSpan {
            data: token,
            context: self.original,
            range: offset + range.start..offset + range.end,
        }
    }

    fn report(&self, span: Range<usize>, message: &str) -> MarkupError {
        let offset = self.source.as_ptr() as usize - self.original.as_ptr() as usize;
        // 'self.source' is a substr of 'self.original'
        MarkupError::from_range(
            &self.original,
            offset + span.start..offset + span.end,
            message.to_string(),
        )
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
