//run: cargo test -- --nocapture

use crate::errors::MarkupLineError;
use crate::messages;
use crate::constants::{WHITESPACE, SEPARATOR};
use super::DEV_PRINT;


/******************************************************************************
 * Macros
 */
macro_rules! devprint {
    ($fmt:literal, $var:expr) => {
        if DEV_PRINT {
            println!($fmt, $var);
        }
    };
}

macro_rules! define_syntax {
    ($count_fsm:ident, $lex_fsm:ident, $tokens:ident,
        $($state:ident {
            $($pat:pat $(if $if_pat:expr)? => $counter:expr, $runner:expr;)*
        })*) =>
    {
        enum State {
            $($state,)*
        }

        //#[allow(unused_variables)]
        fn count_memory_needed(
            $count_fsm: &mut FileIter,
            item: <FileIter as Iterator>::Item,
        ) -> LexerCapacities {
            match $count_fsm.state {
                $(State::$state => match item {
                    $($pat $(if $if_pat)* => $counter,)*
                },)*
            }
        }

        fn step<'a>(
            $lex_fsm: &mut FileIter<'a>,
            $tokens: &mut LexemeLists<'a>,
            item: <FileIter as Iterator>::Item,
        ) {
            match $lex_fsm.state {
                $(State::$state => match item {
                    $($pat $(if $if_pat)* => $runner,)*
                },)*
            }
        }
    };
}


/******************************************************************************
 * Syntax
 */
#[derive(Debug)]
pub enum HeadLexeme<'a> {
    Key(&'a str),
    Separator,
    Blank,
    GroupBegin,
    GroupClose,
}
#[derive(Debug)]
pub enum BodyLexeme<'a> {
    Section(&'a str),
    Separator,
    GroupBegin,
    GroupClose,
}

define_syntax! { count_fsm, lex_fsm, tokens,
    Head {
        ('|', index, _) =>
            {
                devprint!("head |{:?}| 1", count_fsm.cursor_move_to(index));
                count_fsm.state = State::Body;
                (0, 1, 0)
            }, {
                let keystr = lex_fsm.cursor_move_to(index);
                tokens.head_push_key(keystr);
                lex_fsm.cursor_move_to(index + '|'.len_utf8());
                lex_fsm.state = State::Body;
            };

        ('{', index, _) =>
            {
                if let Some(('{', _, _)) = count_fsm.next() { // Second '{'
                    devprint!("head |{:?}| 2", count_fsm.cursor_move_to(index));
                    count_fsm.eat_charlist(&SEPARATOR);
                    count_fsm.state = State::HeadBrackets;
                    (0, 2, 0)
                } else {
                    panic!(messages::MISSING_LBRACKET);
                }
            }, {
                let keystr = lex_fsm.cursor_move_to(index);
                tokens.head_push_key(keystr);

                lex_fsm.next(); // Skip second '{'
                lex_fsm.eat_charlist(&SEPARATOR);
                lex_fsm.cursor_move_to(lex_fsm.rindex);
                tokens.heads.push(HeadLexeme::GroupBegin);
                lex_fsm.state = State::HeadBrackets;
            };
        (';', index, _) =>
            {
                devprint!("head |{:?}| 2", count_fsm.cursor_move_to(index));
                count_fsm.eat_charlist(&SEPARATOR);
                (0, 2, 0)
            }, {
                let keystr = lex_fsm.cursor_move_to(index);
                tokens.head_push_key(keystr);
                lex_fsm.eat_charlist(&SEPARATOR);
                lex_fsm.cursor_move_to(lex_fsm.rindex);
                tokens.heads.push(HeadLexeme::Separator);
            };


        (ch, index, _) if SEPARATOR.contains(&ch) =>
            {
                devprint!("head |{:?}| 1", count_fsm.cursor_move_to(index));
                count_fsm.eat_charlist(&WHITESPACE);
                (0, 1, 0)
            }, {
                let keystr = lex_fsm.cursor_move_to(index);
                tokens.head_push_key(keystr);
                lex_fsm.eat_charlist(&WHITESPACE);
                lex_fsm.cursor_move_to(lex_fsm.rindex);
            };

        (',', _, _) => panic!(messages::HEAD_COMMA_OUTSIDE_BRACKETS), {};
        _ => (0, 0, 0), {};
    }

    HeadBrackets {
        ('}', index, _) =>
            {
                if let Some(('}', _, _)) = count_fsm.next() { // second '}'
                    devprint!("hb   |{:?}| 2", count_fsm.cursor_move_to(index));
                    count_fsm.eat_charlist(&SEPARATOR);
                    count_fsm.state = State::Head;
                    (0, 2, 0)
                } else {
                    panic!(messages::MISSING_RBRACKET);
                }
            }, {
                let keystr = lex_fsm.cursor_move_to(index);
                tokens.head_push_key(keystr);

                lex_fsm.next(); // Skip second '}'
                lex_fsm.eat_charlist(&SEPARATOR);
                lex_fsm.cursor_move_to(lex_fsm.rindex);
                tokens.heads.push(HeadLexeme::GroupClose);
                lex_fsm.state = State::Head;
            };

        ('|', _, _) => panic!(messages::HEAD_INVALID_CLOSE), {};
        ('\\', _, _) => panic!(messages::HEAD_NO_ESCAPING), {};
        _ => (0, 0, 0), {};
    }

    Body {
        ('\n', index, rest) if rest.starts_with("\n|") =>
            {
                devprint!("body |{:?}| 1", count_fsm.cursor_move_to(index));
                count_fsm.next(); // Skip '|'
                count_fsm.eat_charlist(&SEPARATOR);
                count_fsm.state = State::Head;
                (0, 0, 1)
            }, {
                let include_newline = lex_fsm.cursor_move_to(index + 1);
                tokens.bodys.push(BodyLexeme::Section(include_newline));
                lex_fsm.next(); // Skip '|'
                lex_fsm.eat_charlist(&SEPARATOR);
                lex_fsm.cursor_move_to(index + "\n|".len());
                lex_fsm.state = State::Head;
            };

        _ => (0, 0, 0), {};
    }
    BodyBrackets { _ => (0, 0, 0), {}; }
}

pub fn process(filestr: &str) -> Result<LexemeLists, MarkupLineError> {
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
        let mut capacity = (0, 0, 0);
        let fsm = &mut FileIter::new(filestr);
        while let Some(item) = fsm.next() {
            let (entries, head, body) = count_memory_needed(fsm, item);
            capacity.0 += entries;
            capacity.1 += head;
            capacity.2 += body;
        }
        capacity
    };
    let mut lexemes = LexemeLists::new(capacity);

    // Lex into lexemes
    {
        let lexemes_ref = &mut lexemes;
        let fsm = &mut FileIter::new(filestr);
        while let Some(item) = fsm.next() {
            //println!("{} {:?} {:?}", fsm.rindex, item.0, item.2.chars().take(20).collect::<String>());
            step(fsm, lexemes_ref, item);
        }
    }

    assert!(lexemes.heads.len() == capacity.1, "{} != {}", lexemes.heads.len(), capacity.1);
    assert!(lexemes.bodys.len() == capacity.2, "{} != {}", lexemes.bodys.len(), capacity.2);


    Ok(lexemes)
}



/******************************************************************************
 * Lexer Control-Flow Structs
 */
type LexerCapacities = (usize, usize, usize);

// TODO: implement iterator and make these not public
pub struct LexemeLists<'a> {
    pub entries: Vec<(usize, usize)>,
    pub heads: Vec<HeadLexeme<'a>>,
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
        } else {
            self.heads.push(HeadLexeme::Key(keystr));
        }
    }
}



struct FileIter<'a> {
    source: &'a str,
    state: State,
    walker: std::str::Chars<'a>,
    peek: Option<char>,
    rest: &'a str,
    rindex: usize,
    cursor: usize,
    cursor_width: usize,
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
            cursor_width: 0,
        }
    }
    fn cursor_move_to(&mut self, index: usize) -> &'a str {
        let from = self.cursor;
        self.cursor = index;
        &self.source[from..index]
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
}

impl<'a> Iterator for FileIter<'a> {
    type Item = (char, usize, &'a str);
    fn next(&mut self) -> Option<Self::Item> {
        let item  = (self.peek?, self.rindex, self.rest);
        self.rest = self.walker.as_str();
        self.rindex += item.0.len_utf8();
        self.peek = self.walker.next();
        Some(item)
    }
}
