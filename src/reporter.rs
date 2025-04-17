// Provides an interface for `.with_context(...)` and error reporting, for
// parser errors especially, similar to how rustc reports
//
// There were two goals with this approach to error handling
// 1. Learn the Rust way to do error handling.
// 2. Learn how to allocate exact amounts for when I make an involved processor
// 3. It is probably fast to pre-allocate. At least it's more memory efficient.
//
// # Errors the Rust way (Citing RustConf 2020 Jane Lusby talk)
// The core concept I'm learning is the separation of
// 1. Error, the cause/reason an error occurred (just message field)
// 2. Gathering Context (other fields of the error structs + error enum tuple)
//   - TODO: `with_context()` method
//   - Error trait impl (I think this is not relevant for this project)
// 3. Propagating errors (? operator)
// 4. Reporting error (fmt::Display)
//
// Although `println!()` has an internal buffer that makes pre-allocation
// unnecessary, I wanted to do it manually for educational purposes.
//run: cargo test -- --nocapture

use std::{cmp, error, fmt, io, mem};
use unicode_width::UnicodeWidthStr;
//use unicode_segmentation::UnicodeSegmentation;

// (64 * 3 / 10 + 1 = 20) 20 for 64bit, (32 * 3 / 10 + 1 = 10) 10 for 32bit
// 0.302 is an overestimate of log(10)/log(2) to err on side of bigger
const USIZE_BASE_10_MAX_DIGITS: usize = mem::size_of::<usize>() * 8 * 302 / 1000 + 1;
const DISPLAY_LIMIT: usize = 20;
const ELLIPSIS: &str =  " ...";

#[test]
fn limit_is_big_enough() {
    assert!(DISPLAY_LIMIT >= ELLIPSIS.len())
}

#[derive(Debug)]
pub enum CliError {
    Markup(MarkupError),
    Io(io::Error),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            CliError::Markup(ref err) => f.write_str(err.to_string().as_str()),
            CliError::Io(ref err) => f.write_str(err.to_string().as_str()),
        }
    }
}
impl error::Error for CliError {}

// This macro is for ergonomics, capacity and str can be specified on one line
// This then calculates total capacity, allocates, then pushes
macro_rules! precalculate_capacity_and_build {
    ($buffer:ident, { $($size:expr => $push:expr;)* }) => {
        let capacity = 0 $(+ $size)*;
        $buffer = String::with_capacity(capacity);

        $($push;)*
        debug_assert!(
            capacity == $buffer.len(),
            "Pre-calculated capacity is incorrect. Off by {}",
            $buffer.len() - capacity
        );
    };
}

#[derive(Debug)]
pub struct MarkupError {
    source: String,
    range: (usize, usize),
    message: String,
}

impl error::Error for MarkupError {}

impl MarkupError {
    pub fn new<'a>(source: &'a str, span: &'a str, message: String) -> Self {
        let index = index_of_substr(source, span);
        if span.is_empty() {
            panic!("Errors on the");
        }
        Self {
            source: source.to_string(),
            range: (index, index + span.len() - 1),
            message,
        }
    }
}

fn parse_row_col_of_range(source: &str, range: (usize, usize)) -> (usize, usize, usize, usize) {
    let seed = (0, 0, range.0, 0, range.1);
    let (_, r1, c1, r2, c2) = source.lines().fold(seed, |mut acc, line| {
        let (line_begin, r1, c1, r2, c2) = &mut acc;

        // Does not matter if line_begin exceeds source
        // This would happen if source does not end with newline
        let len = line.len() + '\n'.len_utf8();
        *line_begin += len;

        if *line_begin < range.0 {
            *r1 += 1;
            *c1 = range.0 - *line_begin;
        }
        if *line_begin < range.1 {
            *r2 += 1;
            *c2 = range.1 - *line_begin;
        }
        acc
    });
    (r1, c1, r2, c2)
}

// TODO: Delegating print source and rows to error enum `CliError`
impl fmt::Display for MarkupError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (r1, c1, r2, c2) = parse_row_col_of_range(&self.source, self.range);
        let r1_digit_len = count_digits(r1);
        let row_digit_len = count_digits(r2);
        let (c1_digit_len, c2_digit_len) = (count_digits(c1), count_digits(c2));

        let is_single_line = r1 == r2;
        let middle_row_count = if is_single_line { 0 } else { r2 - r1 - 1 };

        let begin_line = self.source.lines().nth(r1).unwrap();
        let close_line = self.source.lines().nth(r2).unwrap();
        let begin_span_width = {
            let begin_line_index = index_of_substr(&self.source, begin_line);
            let span_begin = self.range.0 - begin_line_index;
            let spaces_width = begin_line[0..span_begin].width_cjk();
            if is_single_line {
                (
                    spaces_width,
                    self.source[self.range.0..self.range.1].width_cjk(),
                )
            } else {
                (
                    spaces_width,
                    begin_line.width_cjk() - spaces_width + '\n'.len_utf8(),
                )
            }
        };
        let close_span_width = if is_single_line {
            0
        } else {
            let span_close = self.range.1 - index_of_substr(&self.source, close_line);
            cmp::min(close_line[0..span_close].width_cjk(), DISPLAY_LIMIT)
        };

        let mut buffer: String;
        precalculate_capacity_and_build!(buffer, {
            // Print the filename, row, and col span
            row_digit_len => pad(&mut buffer, row_digit_len, "");
            4 => buffer.push_str("--> ");
            8 => buffer.push_str("<source>");
            1 => buffer.push(':');
            // Rows-col range
            r1_digit_len => push_num(&mut buffer, count_digits(r1), r1);
            1 => buffer.push(':');
            c1_digit_len => push_num(&mut buffer, c1_digit_len, c1);
            row_digit_len => push_num(&mut buffer, row_digit_len, r1);
            1 => buffer.push(':');
            c2_digit_len => push_num(&mut buffer, c2_digit_len, c2);
            1 => buffer.push('\n');

            // Beginning padding for source code
            row_digit_len => pad(&mut buffer, row_digit_len, "");
            3 => buffer.push_str(" |\n");

            // Error marker for begin line
            row_digit_len => pad(&mut buffer, row_digit_len, "");
            3 => buffer.push_str(" | ");
            begin_span_width.0 + begin_span_width.1 * 'v'.len_utf8() => {
                for _ in 0..begin_span_width.0 {
                    buffer.push(' ');
                }
                for _ in 0..begin_span_width.1 {
                    buffer.push('v');
                }
            };
            1 => buffer.push('\n');

            // Print begin line with row numbers
            row_digit_len => push_num(&mut buffer, row_digit_len, r1);
            3 => buffer.push_str(" | ");
            begin_line.len() => buffer.push_str(begin_line);
            1 => buffer.push('\n');

            // Print source with row numbers
            {
                self.source
                    .lines()
                    .skip(r1 + 1)
                    .take(middle_row_count)
                    .map(|line| row_digit_len + 3 + line.len() + 1)
                    .sum::<usize>()
            } => {
                let mut row = r1 + 1;
                for line in self.source.lines().skip(r1 + 1).take(middle_row_count) {
                    push_num(&mut buffer, row_digit_len, row);
                    buffer.push_str(" | ");
                    buffer.push_str(line);
                    buffer.push('\n');
                    row += 1;
                }
            };

            // Print close source with row numbers
            row_digit_len => push_num(&mut buffer, row_digit_len, r2);
            3 => buffer.push_str(" | ");
            if is_single_line { 0 } else { close_line.len() }
                => if !is_single_line { buffer.push_str(close_line); };
            1 => buffer.push('\n');

            // Error marker for close line
            row_digit_len => pad(&mut buffer, row_digit_len, "");
            3 => buffer.push_str(" | ");
            close_span_width => {
                for _ in 0..close_span_width {
                    buffer.push('^');
                }
            };
            1 => buffer.push('\n');

            // Closing padding for source code
            row_digit_len => pad(&mut buffer, row_digit_len, "");
            3 => buffer.push_str(" |\n");

            // Print error message
            row_digit_len => pad(&mut buffer, row_digit_len, "");
            3 => buffer.push_str(" = ");
            self.message.len() => buffer.push_str(self.message.as_str());
        });
        //for line in self.source.lines().skip(r1).take(r2 + 1) {
        //    println!("{:?}", trim_to_limit(line));
        //}

        //println!("{:?}", (r1, c1, r2, c2));
        //println!("{:?}", &self.source.lines().nth(1).unwrap()[19..]);
        //println!("{:?}", &self.source.lines().nth(3).unwrap()[..c2 + 1]);

        f.write_str(buffer.as_str())
    }
}

/*******************************************************************************
 * For printing
 **/

fn index_of_substr<'a>(source: &'a str, substr: &'a str) -> usize {
    (substr.as_ptr() as usize) - (source.as_ptr() as usize)
}

// If I do include this, need to deal with error markers are trimmed substrings
//fn trim_to_limit(line: &str, is_reverse: bool) -> (&str, &str) {
//    let len = line.width_cjk();
//    if len > DISPLAY_LIMIT {
//        let mut width = len + ELLIPSIS.len();
//        let mut close = 0;
//        if is_reverse {
//            for (index, grapheme) in line.grapheme_indices(true).rev() {
//                width -= grapheme.width_cjk();
//                if width <= DISPLAY_LIMIT {
//                    close = index;
//                    break
//                }
//            }
//        } else {
//            for (index, grapheme) in line.grapheme_indices(true) {
//                width -= grapheme.width_cjk();
//                if width <= DISPLAY_LIMIT {
//                    close = index;
//                    break
//                }
//            }
//        }
//        (&line[0..close], ELLIPSIS)
//    } else {
//        (line, "")
//    }
//}

// Although the standard library provides this function (`usize::to_string`)
// I want no-allocations version (mostly as an intellectual exercise)
// similar to https://github.com/rust-lang/rust/blob/1.26.0/src/libcore/fmt/num.rs#L50-L98
fn push_num(buffer: &mut String, digit_len: usize, mut num: usize) {
    let mut temp: [u8; USIZE_BASE_10_MAX_DIGITS] =
        unsafe { mem::MaybeUninit::uninit().assume_init() };
    let mut curr = temp.len();
    let base = 10;
    loop {
        // do-while num != 0
        let remainder: u8 = (num % base) as u8;
        num /= base;
        curr -= 1;
        temp[curr] = remainder + 48;
        if num == 0 {
            break;
        }
    }
    let numstr = unsafe { std::str::from_utf8_unchecked(&temp[curr..]) };
    assert!(numstr.len() <= digit_len);
    pad(buffer, digit_len, numstr);
}

fn count_digits(mut num: usize) -> usize {
    let mut size = 0;
    let base = 10;
    loop {
        num /= base;
        size += 1;
        if num == 0 {
            break;
        }
    }
    size
}

fn pad(buffer: &mut String, len: usize, to_pad: &str) {
    assert!(to_pad.len() <= len);
    buffer.push_str(to_pad);
    for _ in 0..len - to_pad.len() {
        buffer.push(' ');
    }
}
