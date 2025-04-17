//run: cargo test -- --nocapture

use crate::constants::{KEYCODES, KEYSTR_UTF8_MAX_LEN, MODIFIERS, SEPARATOR};
use crate::errors;
use crate::reporter::MarkupError;
use std::ops::Range;

/******************************************************************************
 * Macros
 */
macro_rules! define_syntax {
    ($fsm:ident, $tokens:ident, $head_cursor:ident, $body_cursor:ident, $is_push:ident,
        $($state:ident {
            $(
                $( $pattern:pat )|+ $( if $guard:expr )? => $runner:expr;
            )*
        })*
    ) => {
        enum State {
            $($state,)*
        }

        fn lex_syntax<'a>(
            $fsm: &mut FileIter<'a>,
            ($tokens, $head_cursor, $body_cursor): &mut LexemeStream<'a>,
            item: <FileIter as Iterator>::Item,
            $is_push: bool,
        ) -> Result<LexerCapacities, MarkupError> {
            match $fsm.state {
                $(State::$state => match item {
                    $( $( $pattern )|+ $( if $guard )? => {
                        $runner
                    })*
                },)*
            }
        }
    };
}

/******************************************************************************
 * Token definitions
 */
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
pub struct LexemeLists<'filestr> {
    entries: Vec<(Range<usize>, Range<usize>)>,
    head: Vec<HeadLexeme>,
    body: Vec<BodyLexeme<'filestr>>,
}

impl<'filestr> LexemeLists<'filestr> {
    fn new(capacity: LexerCapacities) -> Self {
        Self {
            entries: Vec::with_capacity(capacity.0),
            head: Vec::with_capacity(capacity.1),
            body: Vec::with_capacity(capacity.2),
        }
    }

    fn head_push_key(
        &mut self,
        fsm: &FileIter<'filestr>,
        keystr: &'filestr str,
    ) -> Result<(), MarkupError> {
        if keystr.is_empty() {
            self.head.push(HeadLexeme::Blank);
        } else if let Some(i) = MODIFIERS.iter().position(|x| *x == keystr) {
            self.head.push(HeadLexeme::Mod(i));
        } else if let Some(i) = KEYCODES.iter().position(|x| *x == keystr) {
            self.head.push(HeadLexeme::Key(i));
        } else {
            fsm.report(keystr, errors::HEAD_INVALID_KEY)?;
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

/******************************************************************************
 * Syntax
 */
// @TODO: Add test for Byte-Order Mark (BOM) ?
pub fn process(filestr: &str) -> Result<LexemeLists, MarkupError> {
    // Skip until first '|' at beginning of line
    let filestr = {
        let mut walker = filestr.chars();
        let mut prev_ch = '\n';
        loop {
            match walker.next() {
                Some('|') if prev_ch == '\n' => break,
                Some(ch) => prev_ch = ch,
                None => return Ok(LexemeLists::new((0, 0, 0))),
            }
        }
        walker.as_str()
    };

    // Calculate the memory needed for the Arrays
    let capacity = {
        // @TODO figure out a way to not do this unnecessary allocation
        let temp = &mut (LexemeLists::new((0, 0, 0)), Cursor(0), Cursor(0));
        //let temp = std::ptr::null() as &mut LexemeLists;

        let mut capacity = (1, 0, 1); // State::Body end not processed in loop
        let fsm = &mut FileIter::new(filestr);
        while let Some(item) = fsm.next() {
            let (entries, head, body) = lex_syntax(fsm, temp, item, false)?;
            capacity.0 += entries;
            capacity.1 += head;
            capacity.2 += body;
        }

        match fsm.state {
            State::Head => panic!("Head not closed"),
            State::HeadBrackets => panic!("Head bracket not closed"),
            State::Body => {} // Then add 1
            State::BodyLiteral => panic!("Body literal not closed"),
            State::BodyBrackets => panic!("Body brackets not closed"),
        }
        capacity
    };
    //println!("{:?}", capacity);

    // Lex into lexemes
    let lexemes = {
        let mut owner = (LexemeLists::new(capacity), Cursor(0), Cursor(0));
        let lexemes_ref = &mut owner;
        let fsm = &mut FileIter::new(filestr);
        while let Some(item) = fsm.next() {
            //println!("{} {:?} {:?}",
            //    fsm.rindex,
            //    item.0,
            //    item.2.chars().take(20).collect::<String>()
            //);
            lex_syntax(fsm, lexemes_ref, item, true)?;
        }

        let last_body = &fsm.source[fsm.cursor.move_to(fsm.source.len())];
        lexemes_ref.0.body.push(BodyLexeme::Section(last_body));
        owner.0.push_entry(&mut owner.1, &mut owner.2);
        owner.0
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
define_syntax! { fsm, tokens, head_cursor, body_cursor, is_push,
    Head {
        (',', i, _) => fsm.report(
            &fsm.source[i..i + ','.len_utf8()],
            errors::HEAD_COMMA_OUTSIDE_BRACKETS
        );

        ('\\', i, _) => fsm.report(
            &fsm.source[i..i + '\\'.len_utf8()],
            errors::HEAD_NO_ESCAPING,
        );

        ('|', i, _) => {
            fsm.state = State::Body;

            let till_close = &fsm.source[fsm.cursor.move_to(i)];
            fsm.cursor.move_to(i + '|'.len_utf8());
            // No eating separator while in State::Body

            if is_push {
                tokens.head_push_key(fsm, till_close)?;
            }
            Ok((0, 1, 0))
        };

        ('{', i, _) => {
            if let Some(('{', _, _)) = fsm.next() { // Second '{'
                fsm.state = State::HeadBrackets;

                let till_bracket = &fsm.source[fsm.cursor.move_to(i)];
                fsm.eat_charlist(&SEPARATOR);
                fsm.cursor.move_to(fsm.rindex);

                if is_push {
                    tokens.head_push_key(fsm, till_bracket)?;
                    tokens.head.push(HeadLexeme::ChoiceBegin);
                }
                Ok((0, 2, 0))
            } else {
                fsm.report(
                    &fsm.source[i + '{'.len_utf8()..i + "{{".len()],
                    errors::MISSING_LBRACKET,
                )
            }
        };

        (ch, i, _) if ch == ';' || SEPARATOR.contains(&ch) => {
            let till_punctuation = &fsm.source[fsm.cursor.move_to(i)];
            fsm.eat_charlist(&SEPARATOR);
            fsm.cursor.move_to(fsm.rindex);

            if is_push {
                tokens.head_push_key(fsm, till_punctuation)?;
                if ch == ';' {
                    tokens.head.push(HeadLexeme::ChordDelim);
                }
            }
            match ch {
                ';' => Ok((0, 2, 0)),
                _ => Ok((0, 1, 0)),
            }
        };


        (_, i, _) => {
            if fsm.cursor.width(i) > KEYSTR_UTF8_MAX_LEN {
                panic!("Panic at the disco")
            } else {
                Ok((0, 0, 0))
            }
        };
    }

    HeadBrackets {
        ('|', i, _) => fsm.report(
            &fsm.source[i..i + '|'.len_utf8()],
            errors::HEAD_INVALID_CLOSE,
        );

        ('\\', i, _) => fsm.report(
            &fsm.source[i..i + '\\'.len_utf8()],
            errors::HEAD_NO_ESCAPING,
        );

        ('}', i, _) => {
            if let Some(('}', _, _)) = fsm.next() { // second '}'
                fsm.state = State::Head;

                let till_bracket = &fsm.source[fsm.cursor.move_to(i)];
                fsm.eat_charlist(&SEPARATOR);
                fsm.cursor.move_to(fsm.rindex);

                if is_push {
                    tokens.head_push_key(fsm, till_bracket)?;
                    tokens.head.push(HeadLexeme::ChoiceClose);
                }
                Ok((0, 2, 0))
            } else {
                fsm.report(
                    &fsm.source[i + '}'.len_utf8()..i + "}}".len()],
                    errors::MISSING_RBRACKET,
                )
            }
        };

        (ch, i, _) if ch == ';' || ch == ',' || SEPARATOR.contains(&ch) => {
            let till_punctuation= &fsm.source[fsm.cursor.move_to(i)];
            fsm.eat_charlist(&SEPARATOR);
            fsm.cursor.move_to(fsm.rindex);
            if is_push {
                tokens.head_push_key(fsm, till_punctuation)?;
                match ch {
                    ';' => tokens.head.push(HeadLexeme::ChordDelim),
                    ',' => tokens.head.push(HeadLexeme::ChoiceDelim),
                    _ => {}
                }
            }
            match ch {
                ';' | ',' => Ok((0, 2, 0)),
                _ => Ok((0, 1, 0)),
            }
        };

        (_, i, _) => {
            if fsm.cursor.width(i) > KEYSTR_UTF8_MAX_LEN {
                panic!("Panic at the disco")
            } else {
                Ok((0, 0, 0))
            }
        };
    }

    // No eating separator while in State::Body
    Body {
        ('\n', i, rest) if rest.starts_with("\n|") => {
            fsm.state = State::Head;

            let include_newline = &fsm.source[fsm.cursor.move_to(i)];
            fsm.next(); // Skip '|'
            fsm.eat_charlist(&SEPARATOR); // Eat cause in State::Head
            fsm.cursor.move_to(i + "\n|".len());

            if is_push {
                tokens.body.push(BodyLexeme::Section(include_newline));
                tokens.push_entry(head_cursor, body_cursor);
            }
            Ok((1, 0, 1))
        };

        (_, i, rest) if rest.starts_with("{{{") => {
            fsm.state = State::BodyLiteral;

            let till_bracket = &fsm.source[fsm.cursor.move_to(i)];
            fsm.next(); // Skip second '{'
            fsm.next(); // Skip third '{'
            fsm.cursor.move_to(fsm.rindex);

            if is_push {
                tokens.body.push(BodyLexeme::Section(till_bracket));
            }
            Ok((0, 0, 1))
        };

        ('{', i, _) if matches!(fsm.peek, Some('{')) => {
            fsm.state = State::BodyBrackets;

            let till_bracket = &fsm.source[fsm.cursor.move_to(i)];
            fsm.next(); // Skip second '{'
            fsm.cursor.move_to(fsm.rindex);

            if is_push {
                tokens.body.push(BodyLexeme::Section(till_bracket));
                tokens.body.push(BodyLexeme::ChoiceBegin);
            }
            Ok((0, 0, 2))
        };

        _ => Ok((0, 0, 0));
    }

    BodyLiteral {
        ('\\', i, _) => {
            let till_backslash = &fsm.source[fsm.cursor.move_to(i)];
            match fsm.next() {
                Some(('\n', _, _)) => {
                    fsm.cursor.move_to(fsm.rindex);
                    tokens.body.push(BodyLexeme::Section(till_backslash));
                    Ok((0, 0, 1))
                }
                Some((ch, _, _)) => {
                    fsm.cursor.move_to(fsm.rindex);
                    let escaped = &fsm.source[i..i + ch.len_utf8()];
                    tokens.body.push(BodyLexeme::Section(till_backslash));
                    tokens.body.push(BodyLexeme::Section(escaped));
                    Ok((0, 0, 2))
                }
                // Let the final '\\' exist by itself
                None => Ok((0, 0, 0)),
            }
        };

        (_, i, rest) if rest.starts_with("}}}") => {
            fsm.state = State::Body;

            let till_bracket = &fsm.source[fsm.cursor.move_to(i)];
            fsm.next(); // Skip second '}'
            fsm.next(); // Skip third '}'
            fsm.cursor.move_to(fsm.rindex);

            if is_push {
                tokens.body.push(BodyLexeme::Section(till_bracket));
            }
            Ok((0, 0, 1))
        };

        _ => Ok((0, 0, 0));
    }

    BodyBrackets {
        ('}', i, _) if matches!(fsm.peek, Some('}')) => {
            fsm.state = State::Body;

            let till_bracket = &fsm.source[fsm.cursor.move_to(i)];
            fsm.next(); // Skip second '}'
            fsm.cursor.move_to(fsm.rindex);

            if is_push {
                tokens.body.push(BodyLexeme::Section(till_bracket));
                tokens.body.push(BodyLexeme::ChoiceClose);
            }
            Ok((0, 0, 2))
        };

        ('\\', i, _) => {
            let till_backslash = &fsm.source[fsm.cursor.move_to(i)];
            match fsm.next() {
                Some(('\n', _, _)) => {
                    fsm.cursor.move_to(fsm.rindex);
                    tokens.body.push(BodyLexeme::Section(till_backslash));
                    Ok((0, 0, 1))
                }
                Some((ch, _, _)) => {
                    fsm.cursor.move_to(fsm.rindex);
                    let escaped = &fsm.source[i..i + ch.len_utf8()];
                    tokens.body.push(BodyLexeme::Section(till_backslash));
                    tokens.body.push(BodyLexeme::Section(escaped));
                    Ok((0, 0, 2))
                }
                // Let the final '\\' exist by itself
                None => Ok((0, 0, 0)),
            }
        };

        (',', i, _) => {
            let till_comma = &fsm.source[fsm.cursor.move_to(i)];
            fsm.cursor.move_to(fsm.rindex);
            tokens.body.push(BodyLexeme::Section(till_comma));
            tokens.body.push(BodyLexeme::ChoiceDelim);
            Ok((0, 0, 2))
        };

        _ => Ok((0, 0, 0));
    }
}

/******************************************************************************
 * Lexer Control-Flow Structs
 */

struct Cursor(usize);
impl Cursor {
    fn move_to(&mut self, index: usize) -> Range<usize> {
        debug_assert!(index >= self.0);
        let from = self.0;
        self.0 = index;
        from..index
    }
    fn width(&self, index: usize) -> usize {
        debug_assert!(index >= self.0);
        index - self.0
    }
}

type LexerCapacities = (usize, usize, usize);
type LexemeStream<'a> = (LexemeLists<'a>, Cursor, Cursor);

// This owns the data that represents our token streams
// Info specific to the 'to_push' = true branch of the lexer
//
struct FileIter<'a> {
    source: &'a str,
    state: State,
    walker: std::str::Chars<'a>,
    peek: Option<char>,
    rest: &'a str,
    rindex: usize,
    cursor: Cursor,
}
impl<'a> FileIter<'a> {
    fn new(source: &'a str) -> Self {
        let mut walker = source.chars();
        let peek = walker.next();
        FileIter {
            source,
            state: State::Head,
            walker,
            rest: source,
            peek,
            rindex: 0,
            cursor: Cursor(0),
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

    fn report(&self, span: &'a str, message: &str) -> Result<LexerCapacities, MarkupError> {
        Err(MarkupError::new(&self.source, span, message.to_string()))
    }
}

impl<'a> Iterator for FileIter<'a> {
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
