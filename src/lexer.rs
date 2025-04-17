//run: cargo test -- --nocapture

use crate::errors::MarkupLineError;
use crate::messages;

#[derive(Debug)]
enum HeadLexeme<'a> {
    Key(&'a str),
    Separator,
    Blank,
    GroupBegin,
    GroupClose,
}
#[derive(Debug)]
enum BodyLexeme<'a> {
    Section(&'a str),
    Separator,
    GroupBegin,
    GroupClose,
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
}

struct MemoryTracker {
}

struct LexemeLists<'a> {
    entries: Vec<(usize, usize)>,
    heads: Vec<HeadLexeme<'a>>,
    bodys: Vec<BodyLexeme<'a>>,
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

impl<'a> FileIter<'a> {
    fn cursor_move_to(&mut self, index: usize) -> &'a str {
        let from = self.cursor;
        self.cursor = index;
        &self.source[from..index]
    }

    fn eat_whitespace() {
    }
}

impl<'a> LexemeLists<'a> {
    fn head_push_key(&mut self, keystr: &'a str) {
        self.heads.push(HeadLexeme::Key(keystr));
    }
}



macro_rules! define_syntax {
    ($count_fsm:ident, $count_specific:ident, $lex_fsm:ident, $tokens:ident,
        $($state:ident {
            $($pat:pat $(if $if_pat:expr)? => $counter:expr, $runner:expr;)*
        })*) =>
    {
        enum State {
            $($state,)*
        }
        #[allow(unused_variables)]
        fn count(
            $count_fsm: &mut FileIter,
            $count_specific: &mut char,
            item: <FileIter as Iterator>::Item,
        ) -> (usize, usize) {
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

define_syntax! { count_fsm, counter, fsm, tokens,
    Head {
        ('|', index, _) =>
            {
                #[cfg(test)]
                println!("head |{:?}| 1", count_fsm.cursor_move_to(index));
                count_fsm.state = State::Body;
                (1, 0)
            }, {
                let keystr = fsm.cursor_move_to(index);
                tokens.head_push_key(keystr);
                fsm.cursor_move_to(index + '|'.len_utf8());
                fsm.state = State::Body;
            };

        ('{', index, _) =>
            {
                if let Some(('{', _, _)) = count_fsm.next() { // Second '{'
                    #[cfg(test)]
                    println!("head |{:?}| 2", count_fsm.cursor_move_to(index));
                    count_fsm.state = State::HeadBrackets;
                    (2, 0)
                } else {
                    panic!("Need second begin bracket");
                }
            }, {
                let keystr = fsm.cursor_move_to(index);
                tokens.head_push_key(keystr);
                fsm.next(); // Skip second '{'
                fsm.cursor_move_to(index + "{{".len());
                tokens.heads.push(HeadLexeme::GroupBegin);
                fsm.state = State::HeadBrackets;
            };

        //(ch, _, _) if ch.is_whitespace() || ch == '+' => {
        //    (0, 0)
        //}, {
        //};

        (',', _, _) => panic!(messages::HEAD_COMMA_OUTSIDE_BRACKETS), {};
        _ => (0, 0), {};
    }

    HeadBrackets {
        ('}', index, _) =>
            {
                if let Some(('}', _, _)) = count_fsm.next() { // second '}'
                    #[cfg(test)]
                    println!("hb   |{:?}| 2", count_fsm.cursor_move_to(index));
                    count_fsm.state = State::Head;
                    (2, 0)
                } else {
                    panic!("Need second close bracket");
                }
            }, {
                let keystr = fsm.cursor_move_to(index);
                tokens.head_push_key(keystr);
                fsm.next(); // Skip second '}'
                fsm.cursor_move_to(index + "}}".len());
                tokens.heads.push(HeadLexeme::GroupClose);
                fsm.state = State::Head;
            };

        ('|', _, _) => panic!(messages::HEAD_INVALID_CLOSE), {};
        ('\\', _, _) => panic!(messages::HEAD_NO_ESCAPING), {};
        _ => (0, 0), {};
    }
    Body {
        ('\n', index, rest) if rest.starts_with("\n|") =>
            {
                #[cfg(test)]
                println!("body |{:?}| 1", count_fsm.cursor_move_to(index));
                count_fsm.next(); // Skip '\n'
                count_fsm.state = State::Head;
                (0, 1)
            }, {
                let include_newline = fsm.cursor_move_to(index + 1);
                tokens.bodys.push(BodyLexeme::Section(include_newline));
                fsm.next(); // Skip '\n'
                fsm.cursor_move_to(index + "\n|".len());
                fsm.state = State::Head;
            };

        _ => (0, 0), {};
    }
    BodyBrackets { _ => (0, 0), {}; }
}

pub fn process(filestr: &str) -> Result<(), MarkupLineError> {
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


    let (head_count, body_count) = {
        let mut capacity = (0, 0);
        let fsm = &mut FileIter::new(filestr);
        while let Some(item) = fsm.next() {
            let to_add = count(fsm, &mut 'a', item);
            capacity.0 += to_add.0;
            capacity.1 += to_add.1;
        }
        capacity
    };
    println!("{} {}", head_count, body_count);


    let tokens = &mut LexemeLists {
        entries:Vec::with_capacity(100),
        heads: Vec::with_capacity(100),
        bodys: Vec::with_capacity(100),
    };
    {
        let fsm = &mut FileIter::new(filestr);
        while let Some(item) = fsm.next() {
            //println!("{} {:?} {:?}", fsm.rindex, ch, rest.chars().take(20).collect::<String>());
            step(fsm, tokens, item);
        }
    }
    assert!(tokens.heads.len() == head_count, "{} != {}", tokens.heads.len(), head_count);
    assert!(tokens.bodys.len() == body_count, "{} != {}", tokens.bodys.len(), body_count);

    println!("{} {}", tokens.heads.len(), tokens.bodys.len());
    for x in &tokens.heads {
        println!("{:?}", x);
    }
    //println!("{:#?}", tokens.bodys);

    Ok(())
}

