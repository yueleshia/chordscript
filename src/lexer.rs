use crate::constants::SEPARATOR;
use std::mem::{replace, swap};

// For escaping during the lexer phase, State::BEscape
// We introduce a new value with "\n" not in the original text.
// An alternative is changing Lexeme::Choice to accept an 'char' or '&str'
const NEWLINE: &str = "\n";

type Range = std::ops::Range<usize>;
pub type StepError = String;
type PassOutput<'filestr> = Result<Option<Lexeme<'filestr>>, StepError>;

//run: cargo build; time cargo run -- shortcuts-debug -c $XDG_CONFIG_HOME/rc/wm-shortcuts keyspace-list ./keyspace-list.sh api
// run: cargo test

#[derive(Debug)]
pub enum Lexeme<'filestr> {
    Key(&'filestr str),
    HChoice(usize, &'filestr str),

    ChordEndK(&'filestr str),
    ChordEndHC(usize, &'filestr str),

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

pub fn lex(input: &str) -> Result<LexOutput, String> {
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
        fragment_len: 0,
        entry_head_index: 0,
        entry_body_index: 0,
        member_num: 0,
        member_h_max: 0,
        member_b_max: 0,
        h_group_size: (0, 0),
        b_group_size: (0, 0),
    };

    jump_init(&mut fsm)?;
    while let Some(ch) = fsm.walker.next() {
        let maybe_push = match fsm.state {
            State::Head => step_head_placeholder(&mut fsm, ch),
            State::HBrackets => step_h_brackets(&mut fsm, ch),
            State::Body => step_body(&mut fsm, ch),
            State::BBrackets => step_b_brackets(&mut fsm, ch),
            State::BEscape => step_b_escape(&mut fsm, ch),
        }?;
        if let Some(item) = maybe_push {
            fragments.push(item);
        }
        debug_assert_eq!(fragments.len(), fsm.fragment_len);
    }

    match fsm.state {
        State::Body => {
            if let Ok(Some(lexeme)) = fsm.emit_body(&input[fsm.cursor.0..]) {
                fragments.push(lexeme)
            }
            fsm.calculate_entry_size(); // @VOLATILE: After the `emit_body()`
            Ok(())
        }
        State::Head if fsm.is_placeholder => Err(fsm.walker.fmt_err(END_BEFORE_PLACEHOLDER_CLOSE)),
        State::Head => Err(fsm.walker.fmt_err(END_BEFORE_HEAD_CLOSE)),
        State::BEscape | State::HBrackets | State::BBrackets => {
            Err(fsm.walker.fmt_err(END_BEFORE_BRACKET_CLOSE))
        }
    }?;

    //println!("Size {:?}", fsm.size);
    debug_assert_eq!(fragments.len(), fsm.fragment_len);
    //fragments.iter().for_each(|lexeme| println!("- {:?}", lexeme));
    //for entry in &fsm.entry_stats {
    //    let i = entry.lexeme_index;
    //    println!("{:?}", &fragments[i.0..i.1]);
    //    println!("{:?}", &fragments[i.1..i.2]);
    //}

    Ok(LexOutput {
        entry_stats: fsm.entry_stats,
        lexemes: fragments,
        original: input,
    })
}

const END_BEFORE_HEAD_CLOSE: &str = "\
    You did not close the head. Please add a '|'. Alternatively, if you placed \
    '|' intentionally at the start of a line, you may wish to consider the \
    following:\n\
    - '{|\\||}' (literals)
    - '{{\\|}}' (you have to add to each relevant permutation), or
    - '{{|}}' (not necessary to escape the backslash)\n\
    depending on your use case.";
const END_BEFORE_PLACEHOLDER_CLOSE: &str =
    "You did not close the placehoder head. Please add a '!'.";
const END_BEFORE_BRACKET_CLOSE: &str = "\
    Missing a second closing curly brace to close the permutation group. \
    Need '}}' to close. If you want a '}' as output, escape it with backslash \
    like '\\}'.";

#[derive(Debug)]
enum State {
    Head,
    HBrackets,
    Body,
    BBrackets,
    BEscape,
}

// Tracks all the state changes for the lexer
#[derive(Debug)]
struct Fsm<'a> {
    walker: CharsWithIndex<'a>,
    cursor: Cursor,

    original: &'a str,
    is_placeholder: bool,
    state: State,
    old_state: State,

    // See second impl block for explanation of the math
    entry_stats: Vec<PostLexEntry>,
    fragment_len: usize,
    entry_head_index: usize,
    entry_body_index: usize,
    member_num: usize,
    member_h_max: usize,
    member_b_max: usize,
    h_group_size: (usize, usize), // left for in all permutations
    b_group_size: (usize, usize), // right for only certain permutations
}

/******************************************************************************
 * The handlers for each 'State::' of the 'Fsm'
 ******************************************************************************/
#[inline]
fn jump_init<'a>(fsm: &mut Fsm<'a>) -> PassOutput<'a> {
    while let Some(peek) = fsm.walker.peek() {
        match (fsm.walker.curr_char, peek) {
            ('\n', '#') => fsm.walker.eat_till_newline(),
            ('\n', c @ '|' | c @ '!') => {
                fsm.walker.next(); // skip newline (and '|' after break)

                // These are set by default
                //fsm.change_state(State::Head);
                fsm.is_placeholder = c == '!';
                debug_assert!(matches!(fsm.state, State::Head));
                break;
            }

            ('\n', _) => {}
            _ => {
                return Err(fsm.walker.fmt_err(
                    "Valid starting characters for a line are:\n\
                    - '#' (comments),\n\
                    - '!' (placeholders),\n\
                    - '|' (commands)",
                ))
            }
        }
        fsm.walker.next();
    }
    fsm.cursor.move_to(fsm.walker.post);

    Ok(None)
}

fn step_head_placeholder<'a>(fsm: &mut Fsm<'a>, ch: char) -> PassOutput<'a> {
    debug_assert!(!SEPARATOR.contains(&'!'));
    debug_assert!(!SEPARATOR.contains(&'|'));

    match (ch, fsm.walker.peek()) {
        ('\n', Some('#')) => {
            let before_newline = fsm.cursor.range_to(fsm.walker.prev);
            fsm.walker.eat_till_newline();
            fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_head(&fsm.original[before_newline])
        }

        ('{', Some('{')) => {
            fsm.change_state(State::HBrackets);

            let before_bracket = fsm.cursor.move_to(fsm.walker.prev);
            fsm.walker.next(); // Skip second '{'
            fsm.cursor.move_to(fsm.walker.post);
            fsm.member_num = 0;
            fsm.emit_head(&fsm.original[before_bracket])
        }

        (';', _) => {
            let before_semicolon = fsm.cursor.move_to(fsm.walker.prev);
            fsm.walker.eat_separator();
            fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_h_chord(&fsm.original[before_semicolon])
        }

        _ if SEPARATOR.contains(&ch) => {
            let before_blank = fsm.cursor.move_to(fsm.walker.prev);
            fsm.walker.eat_separator();
            fsm.cursor.move_to(fsm.walker.post);
            if let Some(';') = fsm.walker.peek() {
                fsm.emit_h_chord(&fsm.original[before_blank])
            } else {
                fsm.emit_head(&fsm.original[before_blank])
            }
        }

        /*************************************
         * Head- and placeholder-specific code
         *************************************/
        ('|', _) if !fsm.is_placeholder => {
            fsm.change_state(State::Body);

            // Change to State::Body
            let before_bar = fsm.cursor.move_to(fsm.walker.prev);
            let lexeme = fsm.emit_h_chord(&fsm.original[before_bar]);
            fsm.cursor.move_to(fsm.walker.post);
            fsm.mark_body_start(); // @VOLATILE: After `emit_h_chord()`
            lexeme
        }

        ('!', _) if !fsm.is_placeholder => Err(fsm.walker.fmt_err(
            "You are currently defining a head, not a placeholder. \
                Did you mean to use '|' instead?",
        )),
        ('!', _) => {
            fsm.change_state(State::Body);

            // Change to State::Body
            let before_exclaim = fsm.cursor.move_to(fsm.walker.prev);
            let lexeme = fsm.emit_h_chord(&fsm.original[before_exclaim]);
            fsm.cursor.move_to(fsm.walker.post);
            fsm.mark_body_start(); // @VOLATILE: after `emit_p_chord()`
            lexeme
        }
        ('|', _) => Err(fsm.walker.fmt_err(
            "You are currently defining a placeholder, not a head. \
                Did you mean to use '!' instead?",
        )),

        /**********************************/
        ('{', _) => Err(fsm.walker.fmt_err(
            "Missing a second opening curly brace. \
            Need '{{' to start an enumeration",
        )),

        _ => Ok(None),
    }
}

#[inline]
fn step_h_brackets<'a>(fsm: &mut Fsm<'a>, ch: char) -> PassOutput<'a> {
    match (ch, fsm.walker.peek()) {
        ('\n', Some('#')) => {
            let before_newline = fsm.cursor.range_to(fsm.walker.prev);
            fsm.walker.eat_till_newline();
            fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_h_choice(&fsm.original[before_newline])
        }
        ('|', _) => Err(fsm
            .walker
            .fmt_err("Unexpected bar '|'. Close the enumeration first with '}}'")),
        ('\\', _) => Err(fsm.walker.fmt_err(
            "You cannot escape characters with backslash '\\' \
                in the hotkey definition portion",
        )),
        (',', _) => {
            let before_comma = fsm.cursor.move_to(fsm.walker.prev);
            fsm.walker.eat_separator();
            fsm.cursor.move_to(fsm.walker.post); // After ','
            let lexeme = fsm.emit_h_choice(&fsm.original[before_comma]);
            fsm.member_num += 1; // @VOLATILE: After `emit_h_choice()`
            lexeme
        }
        ('}', Some('}')) => {
            fsm.revert_state();

            let before_bracket = fsm.cursor.range_to(fsm.walker.prev);
            fsm.walker.next(); // Skip second '}'
            fsm.walker.eat_separator();
            fsm.member_h_max = fsm.member_h_max.max(fsm.member_num + 1);
            fsm.cursor.move_to(fsm.walker.post); // After second '}'
            fsm.emit_h_choice(&fsm.original[before_bracket])
        }

        (';', _) => {
            let before_semicolon = fsm.cursor.move_to(fsm.walker.prev);
            fsm.walker.eat_separator();
            fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_hc_chord(&fsm.original[before_semicolon])
        }
        // @VOLATILE: This should be the last of the non-errors
        //            in case one of the our symbols is in SEPARATOR
        _ if SEPARATOR.contains(&ch) => {
            let before_blank = fsm.cursor.move_to(fsm.walker.prev);
            fsm.walker.eat_separator();
            fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_h_choice(&fsm.original[before_blank])
        }

        ('}', _) => Err(fsm.walker.fmt_err(
            "Missing a second closing curly brace. \
            Need '}}' to close an enumeration",
        )),
        _ => Ok(None),
    }
}
#[inline]
fn step_body<'a>(fsm: &mut Fsm<'a>, ch: char) -> PassOutput<'a> {
    match (ch, fsm.walker.peek()) {
        // Note: Single '{' is not an error, thus doing a different match
        ('\n', Some('#')) => {
            let before_newline = fsm.cursor.move_to(fsm.walker.prev);
            fsm.walker.eat_till_newline();
            fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_body(&fsm.original[before_newline])
        }

        ('{', Some('{')) => {
            fsm.change_state(State::BBrackets);

            let before_brackets = fsm.cursor.move_to(fsm.walker.prev);
            fsm.walker.next(); // Skip second '{'
            fsm.cursor.move_to(fsm.walker.post);
            fsm.member_num = 0;
            fsm.emit_body(&fsm.original[before_brackets])
        }

        ('\n', Some(c @ '|') | Some(c @ '!')) => {
            fsm.change_state(State::Head);
            fsm.is_placeholder = c == '!';

            // Final newlines will be trimmed at parser stage anyway
            let before_newline = fsm.cursor.move_to(fsm.walker.prev);
            let lexeme = fsm.emit_body(&fsm.original[before_newline]);
            fsm.walker.next(); // Skip '|' or '!'
            fsm.cursor.move_to(fsm.walker.post);
            fsm.calculate_entry_size(); // @VOLATILE: After `emit_body()`
            lexeme
        }

        _ => Ok(None),
    }
}

#[inline]
fn step_b_brackets<'a>(fsm: &mut Fsm<'a>, ch: char) -> PassOutput<'a> {
    match (ch, fsm.walker.peek()) {
        ('\n', Some('#')) => {
            let before_newline = fsm.cursor.move_to(fsm.walker.post);
            fsm.walker.eat_till_newline();
            fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_b_choice(&fsm.original[before_newline])
        }

        ('\\', _) => {
            let before_backslash = fsm.cursor.move_to(fsm.walker.prev);
            fsm.cursor.move_to(fsm.walker.post);
            fsm.change_state(State::BEscape);
            fsm.emit_b_choice(&fsm.original[before_backslash])
        }

        ('|', _) if fsm.walker.last_char == '\n' => Err(fsm.walker.fmt_err(
            "A '|' here conflicts with starting a new entry. \
            Close the enumeration first with '}}'.\n\
            If you want a '|' as the first character in line try:\n\
            - '\\n|' on the previous line or\n\
            - '\\|' escaping it on this line.",
        )),
        (',', _) => {
            let before_comma = fsm.cursor.range_to(fsm.walker.prev);
            let frag = fsm.emit_b_member(&fsm.original[before_comma]);
            fsm.cursor.move_to(fsm.walker.post); // After ','
            fsm.member_num += 1; // @VOLATILE: ensure this is after emit
            frag
        }
        ('}', Some('}')) => {
            let before_bracket = fsm.cursor.range_to(fsm.walker.prev);
            fsm.walker.next(); // Skip the second '}'
            fsm.change_state(State::Body);
            fsm.member_b_max = fsm.member_b_max.max(fsm.member_num + 1);
            fsm.cursor.move_to(fsm.walker.post); // After '}'
            fsm.emit_b_member(&fsm.original[before_bracket])
        }
        _ => Ok(None),
    }
}

#[inline]
fn step_b_escape<'a>(fsm: &mut Fsm<'a>, ch: char) -> PassOutput<'a> {
    fsm.revert_state();
    match ch {
        // Emit the next character
        '\\' | '|' | ',' => {
            let after_escaped = fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_b_choice(&fsm.original[after_escaped])
        }

        // Emit a newline that does not exist in fsm.original
        'n' => {
            fsm.cursor.move_to(fsm.walker.post);
            fsm.emit_b_choice(NEWLINE)
        }

        // Skip newlines
        '\n' => {
            fsm.cursor.move_to(fsm.walker.post);
            Ok(None)
        }
        _ => Err(fsm.walker.fmt_err(
            "This character is not eligible for escaping. \
            You might need to escape a previous '\\'.",
        )),
    }
}

impl<'a> Fsm<'a> {
    #[inline]
    fn change_state(&mut self, target_state: State) {
        #[cfg(debug_assertions)]
        match (&self.state, &target_state) {
            (State::Head, State::Body | State::HBrackets) => {}
            (State::Body | State::HBrackets, State::Head) => {}
            (State::Body, State::BBrackets) => {}
            (State::BBrackets, State::Body) => {}

            (State::BBrackets, State::BEscape) => {}
            (State::BEscape, State::BBrackets) => {}
            (a, b) => unreachable!(
                "\n{}",
                self.walker
                    .fmt_err(format!("Invalid state transition {:?} -> {:?}", a, b).as_str())
            ),
        }
        self.old_state = replace(&mut self.state, target_state);
    }

    #[inline]
    fn revert_state(&mut self) {
        swap(&mut self.state, &mut self.old_state);
    }
}

/******************************************************************************
 * Math for the Finite State Machine ('Fsm')
 ******************************************************************************/
// All the size calculations for pre-allocating the 'Vec' for the parser
// Although they are all only called once or twice, these consolidate the math
//
// There are three calculations:
// 1. The number of brackets groups '{{' '}}' ('member_h_max', 'member_b_max')
//    This is measuring the fattest bracket group
// 2. The count for size required by the parser ('h_group_size', 'b_group_size')
//    For head, this measures conservatively along chord boundaries
//    For body, we can get exact measures
// 3. Member num (the index of the variant of the permutation) is handled
//    in each 'step_()' function
macro_rules! define_emitter {
    ($($fn:ident ($self:ident, $frag:ident ) $expr:expr )*) => {
        $(
            #[inline]
            fn $fn(&mut $self, $frag: &'a str) -> PassOutput<'a> {
                if $frag.is_empty() {
                    Ok(None)
                } else {
                    $self.fragment_len += 1;
                    $expr
                }
            }
        )*
    };
}
impl<'a> Fsm<'a> {
    define_emitter! {
        // Outside of head/placeholder '{{' and '}}'
        emit_head(self, frag) Ok(Some(Lexeme::Key(frag)))
        // Inside of '{{' and '}}'
        emit_h_choice(self, frag) Ok(Some(Lexeme::HChoice(self.member_num, frag)))

        // Before closing '!', closing '|', and ';'
        emit_h_chord(self, frag) {
            self.h_group_size.0 += 1;
            Ok(Some(Lexeme::ChordEndK(frag)))
        }
        emit_hc_chord(self, frag) {
            self.h_group_size.1 += 1;
            Ok(Some(Lexeme::ChordEndHC(self.member_num, frag)))
        }

        // Outside of body '{{' and '}}'
        emit_body(self, frag) {
            self.b_group_size.0 += 1;
            Ok(Some(Lexeme::Literal(frag)))
        }
        // Inside of body '{{' and '}}', when not before ','
        emit_b_choice(self, frag) {
            self.b_group_size.1 += 1;
            Ok(Some(Lexeme::BChoice(self.member_num, frag)))
        }
    }
    // Inside of body '{{' and '}}', when before ','
    #[inline]
    fn emit_b_member(&mut self, frag: &'a str) -> PassOutput<'a> {
        if self.member_num > self.member_h_max {
            Err(self.walker.fmt_err(
                "The number of body permutations cannot exceed the number \
                of head permutations.\n\
                Either delete the highlighted body portion or add more \
                options for the head.\n\
                If you want a comma as a text, you escape like '\\,'.",
            ))
        } else {
            self.emit_b_choice(frag)
        }
    }

    #[inline]
    fn mark_body_start(&mut self) {
        self.entry_body_index = self.fragment_len;
    }

    #[inline]
    fn calculate_entry_size(&mut self) {
        //print!("{:?} {} ", self.h_group_size, self.member_h_max);
        //print!("{:?} {} ", self.b_group_size, self.member_b_max);
        let s = (
            self.h_group_size.0 * self.member_h_max + self.h_group_size.1,
            self.b_group_size.0 * self.member_b_max + self.b_group_size.1,
        );

        // Only possibly equal after push for the last entry
        debug_assert!(self.entry_stats.len() < self.entry_stats.capacity());
        debug_assert!(self.member_h_max >= self.member_b_max);
        self.entry_stats.push(PostLexEntry {
            is_placeholder: self.is_placeholder,
            head_size: s.0,
            body_size: s.1,

            // Head determines the number of permutations because we always
            // have more head permutations than body
            permutations: self.member_h_max,
            head: self.entry_head_index,
            body: self.entry_body_index,
            tail: self.fragment_len,
        });

        self.entry_head_index = self.fragment_len;
        self.member_h_max = 1;
        self.member_b_max = 1;
        self.h_group_size = (0, 0);
        self.b_group_size = (0, 0);
    }
}

#[derive(Debug)]
struct Cursor(usize);
impl Cursor {
    #[inline]
    fn move_to(&mut self, till: usize) -> Range {
        let range = self.range_to(till);
        self.0 = till;
        range
    }

    #[inline]
    fn range_to(&self, till: usize) -> Range {
        self.0..till
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
    row: usize,
    col: usize,

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
            row: 0,
            col: 0,

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

    fn fmt_err(&self, msg: &str) -> String {
        debug_assert!(self.row >= 1);
        let row_string = self.row.to_string();
        let line = self.orig.lines().nth(self.row - 1).unwrap_or("");
        let spaces = " ".repeat(row_string.len());
        let term_width = line.chars().take(self.col).map(|_| 1);
        let arrows = "^".repeat(term_width.sum());
        format!(
            "    {} | {}\n    {}   {}\n{}",
            row_string, line, spaces, arrows, msg
        )
    }
    //fn fmt_span_err(&self, msg: &str) -> Result<(), String> {
    //    debug_assert!(self.row >= 1);
    //    Err("".into())
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

            self.col += 1;
            if self.last_char == '\n' {
                self.row += 1;
                self.col = 1;
            }

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
            assert_eq!($iter.peek, $ev);
            assert_eq!($iter.peek, $ev);
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
