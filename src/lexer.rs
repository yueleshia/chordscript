//run: cargo test -- --nocapture

use crate::constants::SEPARATOR;
use crate::errors::lexer as errors;
use crate::reporter::MarkupError;
use crate::structs::Cursor;

use std::mem::swap;

// For escaping during the lexer phase, State::BEscape
// We introduce a new value with "\n" not in the original text.
// An alternative is changing Lexeme::Choice to accept an 'char' or '&str'
const NEWLINE: &str = "\n";

type Output<T> = Result<T, MarkupError>;
type StepOutput<'filestr> = Output<Option<Lexeme<'filestr>>>;

#[derive(Debug)]
pub enum Lexeme<'filestr> {
    Key(&'filestr str),
    HChoice(usize, &'filestr str),

    ChordDelimH(&'filestr str),
    ChordDelimHC(usize, &'filestr str),

    Literal(&'filestr str),
    BChoice(usize, &'filestr str),
}

#[derive(Debug)]
pub struct LexOutput<'filestr> {
    pub lexemes: Vec<Lexeme<'filestr>>,
    pub entry_stats: Vec<PostLexEntry>,
    pub original: &'filestr str,
}

#[derive(Debug)]
pub struct PostLexEntry {
    pub is_placeholder: bool,
    pub head_size: usize,
    pub body_size: usize,
    pub permutations: usize,
    pub head: usize,
    pub body: usize,
    pub tail: usize,
}

pub fn lex(input: &str) -> Result<LexOutput, MarkupError> {
    let entry_estimate = input
        .lines()
        .map(|line| line.chars().next().unwrap_or(' '))
        .filter(|c| *c == '|' || *c == '!')
        .count();

    let mut fragments: Vec<Lexeme> = Vec::with_capacity(input.len());
    let mut fsm = Fsm {
        walker: CharsWithIndex::new(input),
        cursor: Cursor(0),
        original: input,
        is_placeholder: false,
        state: State::Head,
        old_state: State::Head,

        entry_stats: Vec::with_capacity(entry_estimate),
        entry_head_index: 0,
        entry_body_index: 0,
        member_num: 0,        // index for HChoice/BChoice (to `filter()` on in parser)
        max_permutes: (1, 1), // (head max, body max)

        chord_count: (0, 0), // (outside, inside) permute group '{{' '}}'
        body_count: (0, 0),
    };

    // Start lexing
    step_init(&mut fsm)?;
    while let Some(ch) = fsm.walker.next() {
        let len = fragments.len();
        let maybe_push = match fsm.state {
            State::Head => step_head_placeholder(&mut fsm, ch, len),
            State::HBrackets => step_h_brackets(&mut fsm, ch, len),
            State::Body => step_body(&mut fsm, ch, len),
            State::BBrackets => step_b_brackets(&mut fsm, ch, len),
            State::BEscape => step_b_escape(&mut fsm, ch, len),
        }?;
        if let Some(item) = maybe_push {
            fragments.push(item);
        }
    }

    // The while loop ends before last lexeme in the body is pushed
    match fsm.state {
        State::Body => {
            if let Ok(Some(lexeme)) = fsm.emit_body(&input[fsm.cursor.0..]) {
                fragments.push(lexeme)
            }
            // True/false does not matter here
            fsm.push_entry(false, fragments.len());
            Ok(None)
        }
        State::Head if fsm.is_placeholder => fsm
            .walker
            .error_at_current(errors::END_BEFORE_PLACEHOLDER_CLOSE),
        State::Head => fsm.walker.error_at_current(errors::END_BEFORE_HEAD_CLOSE),
        State::BEscape | State::HBrackets | State::BBrackets => fsm
            .walker
            .error_at_current(errors::END_BEFORE_BRACKET_CLOSE),
    }?;

    //fragments.iter().for_each(|lexeme| println!("- {:?}", lexeme));

    Ok(LexOutput {
        entry_stats: fsm.entry_stats,
        lexemes: fragments,
        original: input,
    })
}

#[derive(Debug)]
enum State {
    Head,
    HBrackets,
    Body,
    BBrackets,
    BEscape,
}

// Finite State Machine
// Tracks all the state changes for the lexer
#[derive(Debug)]
struct Fsm<'a> {
    walker: CharsWithIndex<'a>,
    cursor: Cursor,

    original: &'a str,
    is_placeholder: bool,
    state: State,
    // This is more useful when we have more states
    old_state: State,

    // See second impl block for explanation of the math
    entry_stats: Vec<PostLexEntry>,
    entry_head_index: usize,
    entry_body_index: usize,
    member_num: usize,
    max_permutes: (usize, usize),

    // Counts (outside, inside) permute group '{{' '}}'
    chord_count: (usize, usize), // This follows ';' and '|'/'!'
    body_count: (usize, usize),  // This follows ',' and '}}'
}

/******************************************************************************
 * The handlers for each 'State::' of the 'Fsm'
 ******************************************************************************/
#[inline]
fn step_init<'a>(fsm: &mut Fsm<'a>) -> StepOutput<'a> {
    while let Some(peek) = fsm.walker.peek() {
        match (fsm.walker.curr_char, peek) {
            ('\n', '#') => fsm.walker.eat_till_newline(),
            ('\n', c @ '|' | c @ '!') => {
                debug_assert!(matches!(fsm.state, State::Head));
                fsm.walker.next(); // skip newline (and '|' after break)
                fsm.is_placeholder = c == '!';
                break;
            }

            ('\n', _) => {}
            _ => return fsm.walker.error_at_current(errors::INVALID_LINE_START),
        }
        fsm.walker.next();
    }
    fsm.cursor.move_to(fsm.walker.post);

    Ok(None)
}

fn step_head_placeholder<'a>(fsm: &mut Fsm<'a>, ch: char, lexeme_count: usize) -> StepOutput<'a> {
    debug_assert!(!SEPARATOR.contains(&'!'));
    debug_assert!(!SEPARATOR.contains(&'|'));

    match (ch, fsm.walker.peek()) {
        ('\n', Some('#')) => {
            let before_newline = fsm.cursor.span_to(fsm.walker.prev);
            fsm.walker.eat_till_newline();
            fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_head(&fsm.original[before_newline])
        }

        ('{', Some('{')) => {
            fsm.change_state(State::Head, State::HBrackets);

            let before_bracket = fsm.cursor.move_to(fsm.walker.prev);
            fsm.walker.next(); // Skip second '{'
            fsm.cursor.move_to(fsm.walker.post);
            fsm.member_num = 0;
            fsm.emit_head(&fsm.original[before_bracket])
        }

        (';', _) => {
            // i.e. cursor is pointing at semicolon
            debug_assert!(fsm.cursor.span_to(fsm.walker.prev).is_empty());
            let semicolon = fsm.cursor.move_to(fsm.walker.post);
            fsm.walker.eat_separator();
            fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_h_chord(&fsm.original[semicolon])
        }

        _ if SEPARATOR.contains(&ch) => {
            let before_blank = fsm.cursor.move_to(fsm.walker.prev);
            fsm.walker.eat_separator();
            fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_head(&fsm.original[before_blank])
        }

        /*************************************
         * Head- and placeholder-specific code
         *************************************/
        ('|', _) if !fsm.is_placeholder => {
            fsm.change_state(State::Head, State::Body);

            // Change to State::Body
            // i.e. cursor is pointing at the bar
            debug_assert!(fsm.cursor.span_to(fsm.walker.prev).is_empty());
            let bar = fsm.cursor.move_to(fsm.walker.post);
            let lexeme = fsm.emit_h_chord(&fsm.original[bar]);
            fsm.mark_body_start(lexeme_count + if let Ok(None) = lexeme {
                unreachable!("Changed the `emit_h_chord()` behaviour?")
            } else {
                1
            })?;
            lexeme
        }

        ('!', _) if !fsm.is_placeholder => fsm.walker.error_at_current(errors::EXCLAIM_IN_HEAD),
        ('!', _) => {
            fsm.change_state(State::Head, State::Body);

            // Change to State::Body
            // i.e. cursor is pointing at the exclamation
            debug_assert!(fsm.cursor.span_to(fsm.walker.prev).is_empty());
            let exclaim = fsm.cursor.move_to(fsm.walker.post);
            let lexeme = fsm.emit_h_chord(&fsm.original[exclaim]);
            fsm.mark_body_start(
                lexeme_count
                    + if let Ok(None) = lexeme {
                        unreachable!("Changed the `emit_h_chord()` behaviour?")
                    } else {
                        1
                    },
            )?;
            lexeme
        }
        ('|', _) => fsm.walker.error_at_current(errors::BAR_IN_PLACEHOLDER),

        /**********************************/
        ('{', _) => fsm.walker.error_at_current(errors::MISSING_LBRACKET),

        (_, Some('|' | '!' | ';')) => {
            let before_head_end = fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_head(&fsm.original[before_head_end])
        }
        _ => Ok(None),
    }
}

#[inline]
fn step_h_brackets<'a>(fsm: &mut Fsm<'a>, ch: char, lexeme_count: usize) -> StepOutput<'a> {
    match (ch, fsm.walker.peek()) {
        ('\n', Some('#')) => {
            let before_newline = fsm.cursor.span_to(fsm.walker.prev);
            fsm.walker.eat_till_newline();
            fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_h_choice(&fsm.original[before_newline])
        }
        ('|', _) => fsm.walker.error_at_current(errors::HEAD_INVALID_CLOSE),
        ('\\', _) => fsm.walker.error_at_current(errors::HEAD_NO_ESCAPING),
        (',', _) => {
            let before_comma = fsm.cursor.move_to(fsm.walker.prev);
            fsm.walker.eat_separator();
            fsm.cursor.move_to(fsm.walker.post); // After ','
            let lexeme = fsm.emit_h_choice(&fsm.original[before_comma]);
            fsm.member_num += 1; // @VOLATILE: After `emit_h_choice()`
            lexeme
        }
        ('}', Some('}')) => {
            fsm.change_state(State::HBrackets, State::Head);

            let before_bracket = fsm.cursor.span_to(fsm.walker.prev);
            fsm.walker.next(); // Skip second '}'
            fsm.walker.eat_separator();
            fsm.max_permutes.0 = fsm.max_permutes.0.max(fsm.member_num + 1);
            fsm.cursor.move_to(fsm.walker.post); // After second '}'
            fsm.emit_h_choice(&fsm.original[before_bracket])
        }

        (';', _) => {
            // i.e. cursor is pointing at semicolon
            debug_assert!(fsm.cursor.span_to(fsm.walker.prev).is_empty());
            let semicolon = fsm.cursor.move_to(fsm.walker.post);
            fsm.walker.eat_separator();
            fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_hc_chord(&fsm.original[semicolon])
        }
        // @VOLATILE: This should be the last of the non-errors
        //            in case one of the our symbols is in SEPARATOR
        _ if SEPARATOR.contains(&ch) => {
            let before_blank = fsm.cursor.move_to(fsm.walker.prev);
            fsm.walker.eat_separator();
            fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_h_choice(&fsm.original[before_blank])
        }

        ('}', _) => fsm.walker.error_at_current(errors::MISSING_RBRACKET),

        (_, Some(';')) => {
            let before_chord_end = fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_h_choice(&fsm.original[before_chord_end])
        }

        _ => Ok(None),
    }
}

#[inline]
fn step_body<'a>(fsm: &mut Fsm<'a>, ch: char, lexeme_count: usize) -> StepOutput<'a> {
    match (ch, fsm.walker.peek()) {
        // Note: Single '{' is not an error, thus doing a different match
        ('\n', Some('#')) => {
            let before_newline = fsm.cursor.move_to(fsm.walker.prev);
            fsm.walker.eat_till_newline();
            fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_body(&fsm.original[before_newline])
        }

        ('{', Some('{')) => {
            fsm.change_state(State::Body, State::BBrackets);

            let before_brackets = fsm.cursor.move_to(fsm.walker.prev);
            fsm.walker.next(); // Skip second '{'
            fsm.cursor.move_to(fsm.walker.post);
            fsm.member_num = 0;
            fsm.emit_body(&fsm.original[before_brackets])
        }

        ('\n', Some(c @ '|') | Some(c @ '!')) => {
            fsm.change_state(State::Body, State::Head);

            // Final newlines will be trimmed at parser stage anyway
            let before_newline = fsm.cursor.move_to(fsm.walker.prev);
            let lexeme = fsm.emit_body(&fsm.original[before_newline]);
            fsm.walker.next(); // Skip '|' or '!'
            fsm.cursor.move_to(fsm.walker.post);
            fsm.push_entry(c == '!', lexeme_count + if let Ok(None) = lexeme {
                0
            } else {
                1
            });
            lexeme
        }

        _ => Ok(None),
    }
}

#[inline]
fn step_b_brackets<'a>(fsm: &mut Fsm<'a>, ch: char, lexeme_count: usize) -> StepOutput<'a> {
    match (ch, fsm.walker.peek()) {
        ('\n', Some('#')) => {
            let before_newline = fsm.cursor.move_to(fsm.walker.post);
            fsm.walker.eat_till_newline();
            fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_b_choice(&fsm.original[before_newline])
        }

        ('\\', _) => {
            fsm.change_state(State::BBrackets, State::BEscape);

            let before_backslash = fsm.cursor.move_to(fsm.walker.prev);
            fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_b_choice(&fsm.original[before_backslash])
        }

        ('|', _) if fsm.walker.last_char == '\n' => fsm
            .walker
            .error_at_current(errors::BODY_BRACKET_NO_NEWLINE_BAR),
        (',', _) => {
            let before_comma = fsm.cursor.span_to(fsm.walker.prev);
            // was 'emite_b_member()'
            let frag = fsm.emit_b_choice(&fsm.original[before_comma]);
            fsm.cursor.move_to(fsm.walker.post); // After ','
            fsm.member_num += 1; // @VOLATILE: ensure this is after emit

            if fsm.member_num > fsm.max_permutes.0 {
                fsm.walker
                    .error_at_current(errors::MORE_BODY_THAN_HEAD_PERMUTATIONS)
            } else {
                frag
            }
        }
        ('}', Some('}')) => {
            fsm.change_state(State::BBrackets, State::Body);

            let before_bracket = fsm.cursor.span_to(fsm.walker.prev);
            fsm.walker.next(); // Skip the second '}'
            fsm.max_permutes.1 = fsm.max_permutes.1.max(fsm.member_num + 1);
            fsm.cursor.move_to(fsm.walker.post); // After '}'

            if fsm.max_permutes.1 > fsm.max_permutes.0 {
                fsm.walker
                    .error_at_current(errors::MORE_BODY_THAN_HEAD_PERMUTATIONS)
            } else {
            // was 'emite_b_member()'
                fsm.emit_b_choice(&fsm.original[before_bracket])
            }
        }
        ('{', Some('{')) => fsm
            .walker
            .error_at_current(errors::DOUBLE_LBRACKET_IN_BODY_PERMUTATION_GROUP),
        _ => Ok(None),
    }
}

macro_rules! match_and_build_escapes {
    ($fn:ident ($fsm:ident, $lexeme_count:ident) {
        $( $($char:literal )|* => $do:expr,)*
        _ => $final:expr,
    }) => {
        #[inline]
        fn $fn<'a>($fsm: &mut Fsm<'a>, ch: char, $lexeme_count: usize) -> StepOutput<'a> {
            swap(&mut $fsm.state, &mut $fsm.old_state);
            match ch {
                $( $($char)|* => $do )*
                _ => $final
            }

        }

        #[test]
        fn ensure_valid_escapees() {
            use crate::constants::VALID_ESCAPEES;

            $($(
                assert!(
                    $char == '\n' || VALID_ESCAPEES
                        .iter()
                        .map(|substr| substr.chars().next().unwrap_or('\n'))
                        .find(|c| *c == $char)
                        .is_some(),
                    "Update 'VALID_ESCAPEES' in constants.rs to match \
                    `step_b_escape()`"
                );
            )*)*

        }

    };
}

match_and_build_escapes! {
    step_b_escape(fsm, lexeme_count) {
        '\\' | '|' | ',' => {
            let after_escaped = fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_b_choice(&fsm.original[after_escaped])
        },

        // Emit a newline that does not exist in fsm.original
        'n' => {
            fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_b_choice(NEWLINE)
        },

        // Skip newlines
        '\n' => {
            fsm.cursor.move_to(fsm.walker.post);
            Ok(None)
        },

        _ => fsm.walker.error_at_current(errors::INVALID_ESCAPE),
    }
}

impl<'a> Fsm<'a> {
    // Including 'from_state' and 'into_state' so that when we inline, the
    // match statements will be optimised out
    //
    // TODO: Return fragment_len
    #[inline]
    fn change_state(&mut self, from_state: State, into_state: State) {
        debug_assert!(matches!(&self.state, from_state));

        #[cfg(debug_assertions)]
        match (&from_state, &into_state) {
            (State::Head, State::Body | State::HBrackets) => {}
            (State::Body | State::HBrackets, State::Head) => {}
            (State::Body, State::BBrackets) => {}
            (State::BBrackets, State::Body) => {}

            (State::BBrackets, State::BEscape) => {}
            (State::BEscape, State::BBrackets) => {}
            (a, b) => {
                let m = format!("Invalid state transition {:?} -> {:?}", a, b);
                let err = self.walker.error_at_current(m.as_str()).unwrap_err();
                use crate::deserialise::Print;
                unreachable!("{}", err.to_string_custom());
            }
        }

        self.old_state = from_state;
        self.state = into_state;
    }
}

/******************************************************************************
 * Math for the Finite State Machine ('Fsm')
 ******************************************************************************/
// All the size calculations for pre-allocating the 'Vec' for the parser
// Although they are all only called once or twice, these consolidate the math
//
// There are three calculations:
// 1. The number of members in brackets groups '{{' '}}' ('max_permutes.0',
//    'max_permutes.1'). This is measuring the fattest bracket group
// 2. The count for size required by the parser ('chord_count', 'body_count')
//    For head, this measures conservatively along chord boundaries
//    For body, we can get exact measures
// 3. 'member_num' (the index of the variant of the permutation) is handled
//    in each 'step_()' function
macro_rules! define_emitter {
    ($($(@$check_empty:ident)? $fn:ident ($self:ident, $frag:ident )
        $expr:expr
    )*) => {
        $(
            #[inline]
            fn $fn(&mut $self, $frag: &'a str) -> StepOutput<'a> {
                $(define_emitter!(@$check_empty $frag);)?
                $expr
            }
        )*
    };
    (@check_frag_empty $frag:ident) => {
        if $frag.is_empty() {
            return Ok(None);
        }

    };
    ($frag:ident) => {};
}
impl<'a> Fsm<'a> {
    // All of these increment self.fragment
    define_emitter! {
        // Outside of head/placeholder '{{' and '}}'
        @check_frag_empty emit_head(self, frag)
            Ok(Some(Lexeme::Key(frag)))
        // Inside of '{{' and '}}'
        @check_frag_empty emit_h_choice(self, frag)
            Ok(Some(Lexeme::HChoice(self.member_num, frag)))


        // Before closing '!', closing '|', and ';'
        emit_h_chord(self, frag) {
            debug_assert!(frag == "|" || frag == "!" || frag == ";");
            self.chord_count.0 += 1;
            Ok(Some(Lexeme::ChordDelimH(frag)))
        }
        emit_hc_chord(self, frag) {
            debug_assert!(frag == ";");
            self.chord_count.1 += 1;
            Ok(Some(Lexeme::ChordDelimHC(self.member_num, frag)))
        }

        // Outside of body '{{' and '}}'
        @check_frag_empty emit_body(self, frag) {
            self.body_count.0 += 1;
            Ok(Some(Lexeme::Literal(frag)))
        }
        // Inside of body '{{' and '}}'
        @check_frag_empty emit_b_choice(self, frag) {
            self.body_count.1 += 1;
            Ok(Some(Lexeme::BChoice(self.member_num, frag)))
        }
    }

    #[inline]
    fn mark_body_start(&mut self, lexeme_count: usize) -> Output<()> {
        self.entry_body_index = lexeme_count;

        // + 1 for the 'ChordDelim' for the last '|'/'!'
        if self.entry_head_index + 1 == self.entry_body_index {
            // I assume 'self.walker' is pointing just past the '|'/'!'
            let before_bar = self.walker.prev;
            let including_bar = self.walker.post;
            let till_bar = &self.original[0..before_bar];
            let first_bar = till_bar.rfind(self.walker.curr_char).unwrap();
            let bar_to_bar = &self.original[first_bar..including_bar];

            // TODO: change this into a two pointers?
            Err(MarkupError::from_str(
                self.original,
                bar_to_bar,
                errors::EMPTY_HOTKEY.to_string(),
            ))
        } else {
            Ok(())
        }
    }

    #[inline]
    fn push_entry(&mut self, next_is_placeholder: bool, lexeme_count: usize) {
        debug_assert!(self.entry_stats.len() < self.entry_stats.capacity());
        debug_assert!(self.max_permutes.0 >= self.max_permutes.1);

        //print!("{:?} {} ", self.chord_count, self.max_permutes.0);
        //print!("{:?} {} ", self.body_count, self.max_permutes.1);
        let s = (
            self.chord_count.0 * self.max_permutes.0 + self.chord_count.1,
            self.body_count.0 * self.max_permutes.0 + self.body_count.1,
        );
        //println!("{:?}", self.entry_stats.iter().fold((s.0, s.1), |(h, b), e| {
        //    (h + e.head_size, b + e.body_size)
        //}));

        // Only possibly equal after push for the last entry
        self.entry_stats.push(PostLexEntry {
            is_placeholder: self.is_placeholder,
            head_size: s.0,
            body_size: s.1,

            // Head determines the number of permutations because we always
            // have more head permutations than body
            permutations: self.max_permutes.0,
            head: self.entry_head_index,
            body: self.entry_body_index,
            tail: lexeme_count,
        });
        //println!("{:?}", self.entry_stats.last().unwrap().tail);

        self.is_placeholder = next_is_placeholder;
        self.entry_head_index = lexeme_count;
        self.max_permutes = (1, 1);
        self.chord_count = (0, 0);
        self.body_count = (0, 0);
    }
}

/******************************************************************************
 * A 'std::str::Chars' wrapper for use in 'first_pass()'
 ******************************************************************************/
#[derive(Debug)]
struct CharsWithIndex<'a> {
    pub(self) iter: std::str::Chars<'a>,
    orig: &'a str,

    // These will be equal at the boundaries
    // And they always be on apart, 'prev' having the same index as 'iter'
    prev: usize, // the left border of the current character
    post: usize, // the right border of the current character

    last_char: char,
    pub(self) curr_char: char,
    peek_char: Option<char>,
}
impl<'a> CharsWithIndex<'a> {
    fn new(text: &'a str) -> Self {
        let last_char = ' ';
        let curr_char = '\n';
        debug_assert!(last_char != '\n'); // So 'self.row' starts at 1
                                          // So 'self.last_char' set to '\n' at first call of `self.next()`
        debug_assert!(curr_char == '\n');
        let mut iter = text.chars();
        let peek_char = iter.next();

        Self {
            iter,
            orig: text,
            prev: 0, // equal to 'post' at the beginning
            post: 0,

            last_char,
            curr_char,
            peek_char,
        }
    }

    #[inline]
    fn peek(&self) -> Option<char> {
        self.peek_char
    }

    fn eat_till_newline(&mut self) {
        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            } else {
                self.next();
            }
        }
    }

    fn eat_separator(&mut self) {
        while let Some(peek) = self.peek() {
            if SEPARATOR.contains(&peek) {
                self.next();
            } else {
                break;
            }
        }
    }

    fn error_at_current(&self, msg: &str) -> StepOutput<'a> {
        let current = &self.orig[self.prev..self.post];
        Err(MarkupError::from_str(self.orig, current, msg.to_string()))
    }

    //fn fmt_err(&self, msg: &str) -> String {
    //    debug_assert!(self.row >= 1);
    //    let row_string = self.row.to_string();
    //    let line = self.orig.lines().nth(self.row - 1).unwrap_or("");
    //    let spaces = " ".repeat(row_string.len());
    //    let term_width = line.chars().take(self.col).map(|_| 1);
    //    let arrows = "^".repeat(term_width.sum());
    //    format!(
    //        "    {} | {}\n    {}   {}\n{}",
    //        row_string, line, spaces, arrows, msg
    //    )
    //}
}

//
impl<'a> Iterator for CharsWithIndex<'a> {
    type Item = char;
    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        let cur = self.peek_char;
        self.peek_char = self.iter.next();

        if let Some(c) = cur {
            // This is sound in the first '.next()' case
            // (prev, post) => (0, 0).next() -> (0, 1)
            self.prev = self.post;
            self.post += c.len_utf8();

            self.last_char = self.curr_char;
            self.curr_char = c;

            Some(c)
        } else {
            self.prev = self.post; // Equal when we reach the end
            None
        }
    }
}

#[test]
fn chars_with_index() {
    macro_rules! assert_peek_next {
        ($iter:ident, $ev:expr) => {
            assert_eq!($iter.peek(), $ev);
            assert_eq!($iter.peek(), $ev);
            assert_eq!($iter.next(), $ev);
        };
    }
    let mut iter = CharsWithIndex::new("a");
    assert_peek_next!(iter, Some('a'));
    assert_peek_next!(iter, None);
    assert_peek_next!(iter, None);
    assert_peek_next!(iter, None);
    assert_peek_next!(iter, None);

    let mut iter = CharsWithIndex::new("");
    assert_peek_next!(iter, None);
    assert_peek_next!(iter, None);
    assert_peek_next!(iter, None);
    assert_peek_next!(iter, None);

    let mut iter = CharsWithIndex::new("你m好!!我是y只mao");
    assert_peek_next!(iter, Some('你'));
    assert_peek_next!(iter, Some('m'));
    assert_peek_next!(iter, Some('好'));
    assert_peek_next!(iter, Some('!'));
    assert_peek_next!(iter, Some('!'));
    assert_peek_next!(iter, Some('我'));
    assert_peek_next!(iter, Some('是'));
    assert_peek_next!(iter, Some('y'));
    assert_peek_next!(iter, Some('只'));
    assert_peek_next!(iter, Some('m'));
    assert_peek_next!(iter, Some('a'));
    assert_peek_next!(iter, Some('o'));
    assert_peek_next!(iter, None);
    assert_peek_next!(iter, None);

    let source = "你m好!!我是y只mao";
    let mut iter = CharsWithIndex::new(source);
    while let Some(c) = iter.next() {
        assert_eq!(&c.to_string(), &source[iter.prev..iter.post]);
    }

    // TODO: test peek and eat_whitespace
    //let mut iter = CharsWithIndex::new("你m好!!我");
}
