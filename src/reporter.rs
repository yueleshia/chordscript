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

use std::{error, fmt, io, mem};
use crate::structs::WithSpan;
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
    pub fn from_range(source: &str, range: (usize, usize), message: String) -> Self {
        Self {
            source: source.to_string(),
            range,
            message,
        }
    }
    pub fn from_str<'a>(context: &'a str, span: &'a str, message: String) -> Self {
        let index = index_of_substr(context, span);
        Self {
            source: context.to_string(),
            range: (index, index + span.len()),
            message,
        }
    }

    pub fn from_span_over<T>(from: &WithSpan<T>, till_inclusive: &WithSpan<T>, message: String) -> Self {
        Self {
            source: from.context.to_string(),
            range: WithSpan::span_to_as_range(from, till_inclusive),
            message,
        }
    }
}

fn convert_to_row_indices(source: &str, range: (usize, usize)) -> (usize, usize, usize, usize) {
    let (begin, close) = range;
    assert!(begin <= close, "Range is out of order");

    let (_, r0, c0, r1, c1) = source.lines().take(source.lines().count() - 1).fold(
        (0, 0, begin, 0, close),
        |(i, r0, c0, r1, c1), line| {
            let len = line.len(); //+ '\n'.len_utf8();
            let next_line = i + len + 1;
            (
                next_line,
                if begin >= next_line { r0 + 1 } else { r0 },
                if begin >= next_line { c0 - len } else { c0 },
                // At the end, `next_line = source.len() + 1`, so > is fine
                if close > next_line { r1 + 1 } else { r1 },
                if close > next_line { c1 - len } else { r1 },
            )
        },
    );
    (r0, c0, r1, c1)
}

//#[test]
//fn hello() {
//    let example = "asdlkfjaldskf
//    qwlkejr
//the cat in the hat is nearly there";
//    println!();
//
//    let len = example.len();
//    //let substr = &example[len..len+1];
//    //let index = index_of_substr(example, substr);
//    let range = (len, len + 10);
//    //println!("{} {:?}", &example[len - 2..len], &range);
//    println!(
//        "{}",
//        MarkupError::from_range(example, range, "asdf".to_string())
//    );
//
//    println!();
//}

// 'index' is offset from the beginning of 'line' (might exceed 'line.len()')
//
fn line_offset_to_width(context: &str, line: &str, index: usize) -> usize {
    let base_index = index_of_substr(context, line);
    let offset = index - base_index;
    let line_len = line.len();
    if offset < line_len {
        line[0..offset].width_cjk()
    } else {
        line.width_cjk() + offset - line_len
    }
}

// TODO: Delegating print source and rows to error enum `CliError`
impl fmt::Display for MarkupError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (begin, close) = self.range;
        let (r0, _, r1, _) = convert_to_row_indices(self.source.as_str(), self.range);
        let row_digit_len = count_digits(r1 + 1);

        let is_single_line = r0 == r1;
        let row_count = r1 - r0 + 1;

        let begin_line = self.source.lines().nth(r0).unwrap();
        let close_line = self.source.lines().nth(r1).unwrap();

        let first_spaces = line_offset_to_width(&self.source, begin_line, begin);
        let first_marker = if is_single_line {
            line_offset_to_width(&self.source, begin_line, close) - first_spaces
        } else {
            begin_line.width_cjk() - first_spaces + '\n'.len_utf8()
        };

        let after_marker = line_offset_to_width(&self.source, close_line, close);
        //println!("{:?} {} {}", before_spaces, before_marker, after_marker_width);

        let mut buffer: String;
        precalculate_capacity_and_build!(buffer, {
            // Print the filename, row, and col span
            //row_digit_len => pad(&mut buffer, row_digit_len, "");
            //4 => buffer.push_str("--> ");
            //8 => buffer.push_str("<source>");
            //1 => buffer.push(':');
            //// Rows-col range
            //r0 => push_num(&mut buffer, count_digits(r0), r0);
            //1 => buffer.push(':');
            //c0_digit_len => push_num(&mut buffer, c0_digit_len, c0);
            //row_digit_len => push_num(&mut buffer, row_digit_len, r0);
            //1 => buffer.push(':');
            //c1_digit_len => push_num(&mut buffer, c1_digit_len, c1);
            //1 => buffer.push('\n');

            // Beginning padding for source code
            row_digit_len => pad(&mut buffer, row_digit_len, "");
            3 => buffer.push_str(" |\n");

            // Error marker for begin line
            if is_single_line { 0 } else {
                row_digit_len + " | ".len() + first_spaces + first_marker + '\n'.len_utf8()
            } => if !is_single_line {
                pad(&mut buffer, row_digit_len, "");
                buffer.push_str(" | ");
                debug_assert!(" ".width_cjk() == 1);
                debug_assert!("v".width_cjk() == 1);
                for _ in 0..first_spaces { buffer.push(' '); }
                for _ in 0..first_marker { buffer.push('v'); }
                buffer.push('\n');
            };

            // Print source with row numbers
            {
                self.source
                    .lines()
                    .skip(r0)
                    .take(row_count)
                    .map(|line| row_digit_len + 3 + line.len() + 1)
                    .sum::<usize>()
            } => {
                let mut row = r0 + 1;
                for line in self.source.lines().skip(r0).take(row_count) {
                    push_num(&mut buffer, row_digit_len, row);
                    buffer.push_str(" | ");
                    buffer.push_str(line);
                    buffer.push('\n');
                    row += 1;
                }
            };

            // Error marker for close line
            if is_single_line {
                row_digit_len + " | ".len() + first_spaces + first_marker + '\n'.len_utf8()
            } else {
                row_digit_len + " | ".len() + after_marker + '\n'.len_utf8()
            } => if is_single_line {
                pad(&mut buffer, row_digit_len, "");
                buffer.push_str(" | ");
                debug_assert!(" ".width_cjk() == 1);
                debug_assert!("^".width_cjk() == 1);
                for _ in 0..first_spaces { buffer.push(' '); }
                for _ in 0..first_marker { buffer.push('^'); }
                buffer.push('\n');
            } else {
                pad(&mut buffer, row_digit_len, "");
                buffer.push_str(" | ");
                for _ in 0..after_marker { buffer.push('^'); }
                buffer.push('\n');
            };

            // Closing padding for source code
            row_digit_len => pad(&mut buffer, row_digit_len, "");
            3 => buffer.push_str(" |\n");

            // Print error message
            row_digit_len => pad(&mut buffer, row_digit_len, "");
            3 => buffer.push_str(" = ");
            self.message.len() => buffer.push_str(self.message.as_str());
        });

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
