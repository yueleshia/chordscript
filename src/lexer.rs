//run: cargo test -- --nocapture

use crate::reporter::MarkupError;
use crate::errors;
use crate::constants::{MODIFIERS, KEYCODES, SEPARATOR, KEYSTR_UTF8_MAX_LEN};


/******************************************************************************
 * Macros
 */
macro_rules! define_syntax {
    ($fsm:ident, $tokens:ident, $is_push:ident,
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
            $tokens: &mut LexemeLists<'a>,
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
 * Syntax
 */
#[derive(Debug)]
pub enum HeadLexeme {
    Key(usize),
    Mod(usize),
    ChordDelim,
    ChoiceDelim,
    Blank,
    ChoiceBegin,
    ChoiceClose,
}
#[derive(Debug)]
pub enum BodyLexeme<'a> {
    Section(&'a str),
    Separator,
    ChoiceBegin,
    ChoiceClose,
}

// Basically one glorified match with these three variables as arguments
define_syntax! { fsm, tokens, is_push,
    Head {
        (',', i, _) => fsm.report(i, i + ','.len_utf8(), errors::HEAD_COMMA_OUTSIDE_BRACKETS);
        ('\\', i, _) => fsm.report(i, i + '\\'.len_utf8(), errors::HEAD_NO_ESCAPING);

        ('|', i, _) => {
            let keystr = fsm.cursor_move_to(i);
            fsm.cursor_move_to(i + '|'.len_utf8());
            fsm.state = State::Body;

            if is_push {
                tokens.head_push_key(keystr)
            }
            Ok((0, 1, 0))
        };

        ('{', i, _) => {
            if let Some(('{', _, _)) = fsm.next() { // Second '{'
                let keystr = fsm.cursor_move_to(i);
                fsm.eat_charlist(&SEPARATOR);
                fsm.cursor_move_to(fsm.rindex);
                fsm.state = State::HeadBrackets;

                if is_push {
                    tokens.head_push_key(keystr);
                    tokens.heads.push(HeadLexeme::ChoiceBegin);
                }
                Ok((0, 2, 0))
            } else {
                fsm.report(i + '{'.len_utf8(), i + "{{".len(), errors::MISSING_LBRACKET)
            }
        };

        (ch, i, _) if ch == ';' || SEPARATOR.contains(&ch) => {
            let keystr = fsm.cursor_move_to(i);
            fsm.eat_charlist(&SEPARATOR);
            fsm.cursor_move_to(fsm.rindex);

            if is_push {
                tokens.head_push_key(keystr);
                if ch == ';' {
                    tokens.heads.push(HeadLexeme::ChordDelim);
                }
            }
            match ch {
                ';' => Ok((0, 2, 0)),
                _ => Ok((0, 1, 0)),
            }
        };


        (_, i, _) => {
            if fsm.cursor_width(i) > KEYSTR_UTF8_MAX_LEN {
                panic!("Panic at the disco")
            } else {
                Ok((0, 0, 0))
            }
        };
    }

    HeadBrackets {
        ('|', i, _) => fsm.report(i, i + '|'.len_utf8(), errors::HEAD_INVALID_CLOSE);
        ('\\', i, _) => fsm.report(i, i + '\\'.len_utf8(), errors::HEAD_NO_ESCAPING);

        ('}', i, _) => {
            if let Some(('}', _, _)) = fsm.next() { // second '}'
                let keystr = fsm.cursor_move_to(i);
                fsm.eat_charlist(&SEPARATOR);
                fsm.cursor_move_to(fsm.rindex);
                fsm.state = State::Head;

                if is_push {
                    tokens.head_push_key(keystr);
                    tokens.heads.push(HeadLexeme::ChoiceClose);
                }
                Ok((0, 2, 0))
            } else {
                fsm.report(i + '}'.len_utf8(), i + "}}".len(), errors::MISSING_RBRACKET)
            }
        };

        (ch, i, _) if ch == ';' || ch == ',' || SEPARATOR.contains(&ch) => {
            let keystr = fsm.cursor_move_to(i);
            fsm.eat_charlist(&SEPARATOR);
            fsm.cursor_move_to(fsm.rindex);
            if is_push {
                tokens.head_push_key(keystr);
                match ch {
                    ';' => tokens.heads.push(HeadLexeme::ChordDelim),
                    ',' => tokens.heads.push(HeadLexeme::ChoiceDelim),
                    _ => {}
                }
            }
            match ch {
                ';' | ',' => Ok((0, 2, 0)),
                _ => Ok((0, 1, 0)),
            }
        };

        (_, i, _) => {
            if fsm.cursor_width(i) > KEYSTR_UTF8_MAX_LEN {
                panic!("Panic at the disco")
            } else {
                Ok((0, 0, 0))
            }
        };
    }

    Body {
        ('\n', i, rest) if rest.starts_with("\n|") => {
            let include_newline = fsm.cursor_move_to(i + 1);
            fsm.next(); // Skip '|'
            fsm.eat_charlist(&SEPARATOR);
            fsm.cursor_move_to(i + "\n|".len());
            fsm.state = State::Head;

            if is_push {
                tokens.bodys.push(BodyLexeme::Section(include_newline));
            }
            Ok((0, 0, 1))
        };

        _ => Ok((0, 0, 0));
    }
    BodyBrackets { _ => Ok((0, 0, 0)); }
}

pub fn process(filestr: &str) -> Result<LexemeLists, MarkupError> {
    // Skip until first '|' at beginning of line
    let filestr = {
        let mut walker = filestr.chars();
        let mut ch = '\n';
        while let Some(next_ch) = walker.next() {
            if ch == '\n' && next_ch == '|' {
                break;
            }
            ch = next_ch;
        }
        walker.as_str()
    };


    // Calculate the memory needed for the Arrays
    let capacity = {
        let temp = &mut LexemeLists::new((0, 0, 0));
        //let temp = std::ptr::null() as &mut LexemeLists;
        let mut capacity = (0, 0, 0);
        let fsm = &mut FileIter::new(filestr);
        while let Some(item) = fsm.next() {
            let (entries, head, body) = lex_syntax(fsm, temp, item, false)?;
            capacity.0 += entries;
            capacity.1 += head;
            capacity.2 += body;
        }
        capacity
    };
    //println!("{:?}", capacity);

    // Lex into lexemes
    let mut lexemes = LexemeLists::new(capacity);
    {
        let lexemes_ref = &mut lexemes;
        let fsm = &mut FileIter::new(filestr);
        while let Some(item) = fsm.next() {
            //println!("{} {:?} {:?}", fsm.rindex, item.0, item.2.chars().take(20).collect::<String>());
            lex_syntax(fsm, lexemes_ref, item, true)?;
        }
    }

    debug_assert!(lexemes.heads.len() == capacity.1, "{} != {}", lexemes.heads.len(), capacity.1);
    debug_assert!(lexemes.bodys.len() == capacity.2, "{} != {}", lexemes.bodys.len(), capacity.2);


    Ok(lexemes)
}



/******************************************************************************
 * Lexer Control-Flow Structs
 */
type LexerCapacities = (usize, usize, usize);

// TODO: implement iterator and make these not public
pub struct LexemeLists<'a> {
    pub entries: Vec<(usize, usize)>,
    pub heads: Vec<HeadLexeme>,
    pub bodys: Vec<BodyLexeme<'a>>,
}

impl<'a> LexemeLists<'a> {
    fn new(capacity: LexerCapacities) -> Self {
        Self {
            entries: Vec::with_capacity(capacity.0),
            heads: Vec::with_capacity(capacity.1),
            bodys: Vec::with_capacity(capacity.2),
        }
    }

    fn head_push_key(&mut self, keystr: &'a str) {
        if keystr.is_empty() {
            self.heads.push(HeadLexeme::Blank);
        } else if let Some(i) = MODIFIERS.iter().position(|x| *x == keystr) {
            self.heads.push(HeadLexeme::Mod(i));
        } else if let Some(i) = KEYCODES.iter().position(|x| *x == keystr) {
            self.heads.push(HeadLexeme::Key(i));
        } else {
            panic!("Invalid key. {} ", keystr)
        }
    }

    //fn push_into_head(&mut self, head_lexeme: HeadLexeme<'a>, source: &'a str) {
    //    self.heads.push(source, head_lexeme);
    //}
}

struct FileIter<'a> {
    source: &'a str,
    state: State,
    walker: std::str::Chars<'a>,
    peek: Option<char>,
    rest: &'a str,
    rindex: usize,
    cursor: usize,
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
            cursor: 0,
        }
    }
    fn cursor_move_to(&mut self, index: usize) -> &'a str {
        debug_assert!(index >= self.cursor);
        let from = self.cursor;
        self.cursor = index;
        &self.source[from..index]
    }

    fn cursor_width(&mut self, index: usize) -> usize {
        index - self.cursor
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

    fn report(&self, begin: usize, close: usize, message: &str) -> Result<LexerCapacities, MarkupError> {
        Err(MarkupError::new(&self.source, &self.source[begin..close], message.to_string()))
    }
}

impl<'a> Iterator for FileIter<'a> {
    type Item = (char, usize, &'a str);
    fn next(&mut self) -> Option<Self::Item> {
        let item  = (self.peek?, self.rindex, self.rest);
        let len_utf8 = item.0.len_utf8();
        self.rest = self.walker.as_str();
        self.rindex += len_utf8;
        self.peek = self.walker.next();
        Some(item)
    }
}
