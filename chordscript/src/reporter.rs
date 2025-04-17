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

use unicode_width::UnicodeWidthStr;
//use unicode_segmentation::UnicodeSegmentation;

use std::{error, fmt};

use crate::sidebyside_len_and_push;
use crate::templates::{Consumer, PreallocLen, PreallocPush};

//const DISPLAY_LIMIT: usize = 20;
//const ELLIPSIS: &str = " ..."; // If we decide to column limit
//#[test]
//fn limit_is_big_enough() {
//    assert!(DISPLAY_LIMIT >= ELLIPSIS.len())
//}

//#[derive(Debug)]
//pub enum CliError {
//    Markup(MarkupError),
//    Io(std::io::Error),
//}
//
//impl fmt::Display for CliError {
//    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//        match *self {
//            CliError::Markup(ref err) => f.write_str(err.to_string().as_str()),
//            CliError::Io(ref err) => f.write_str(err.to_string().as_str()),
//        }
//    }
//}
//impl error::Error for CliError {}

#[derive(Debug)]
pub struct MarkupError {
    source: String,
    range: (usize, usize),
    message: String,
}

impl error::Error for MarkupError {}

impl MarkupError {
    pub fn from_str<'a>(context: &'a str, span: &'a str, message: String) -> Self {
        let index = (span.as_ptr() as usize) - (context.as_ptr() as usize);
        Self {
            source: context.to_string(),
            range: (index, index + span.len()),
            message,
        }
    }
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

// @TODO Delegating print source and rows to error enum `CliError`
impl fmt::Display for MarkupError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        //let buffer = &mut String::with_capacity(self.string_len());
        //self.push_string_into(buffer);
        //f.write_str(buffer.as_str())
        self.pipe((), f);

        Ok(())
    }
}

/****************************************************************************
 *
 ****************************************************************************/
type OutputType = ();
//// @TODO: Add colour
//enum OutputType {
//    PosixShell,
//    HTML,
//}

impl PreallocLen<OutputType> for MarkupError {
    fn len(&self, extra: OutputType) -> usize {
        error_len(self, extra)
    }
}
impl<U: Consumer> PreallocPush<OutputType, U> for MarkupError {
    fn pipe(&self, extra: OutputType, buffer: &mut U) {
        error_pipe(self, extra, buffer);
    }
}

sidebyside_len_and_push!(error_len, error_pipe<U>(me: &MarkupError, _extra: OutputType, buffer: U) {
    let context = me.source.as_str();

    let mut iter = context.lines().enumerate().map(|(i, line)| {
        ((line.as_ptr() as usize) - (context.as_ptr() as usize), i, line)
    });
    let (index, row, line) = iter.find(|(index, _, line)| me.range.0 < index + line.len() + "\n".len()).unwrap();
    let start_row = ContextfulRow {
        row_number: row,
        highlight_index: me.range,
        context_index: index,
    };
    let start_input = PaintInput {
        row_number_max_len: count_digits(row),
        line,
        //word_wrap: 100,
        message: &me.message,
    };
} {
    start_row.len(start_input) => start_row.pipe(start_input, buffer);


    // @TODO: Display second row
});

/****************************************************************************
 * For printing a single row
 ****************************************************************************/
struct PaintInput<'a> {
    row_number_max_len: u8,
    line: &'a str,
    //word_wrap: usize,
    message: &'a str,
}

struct ContextfulRow {
    row_number: usize,
    highlight_index: (usize, usize),
    context_index: usize,
}

impl ContextfulRow {
    sidebyside_len_and_push!(len, pipe<U>(self: &Self, extra: PaintInput, buffer: U) {
    } {
        USIZE_BASE_10_MAX_DIGITS as usize => buffer.consume(&PADDING[0..extra.row_number_max_len as usize]);
        " |\n";

        USIZE_BASE_10_MAX_DIGITS as usize => push_num(self.row_number, buffer);
        " | ";
        extra.line.len() => buffer.consume(extra.line);
        "\n";

        USIZE_BASE_10_MAX_DIGITS as usize => buffer.consume(&PADDING[0..extra.row_number_max_len as usize]);
        " | ";
        extra.line.width_cjk() => {
            let offset = self.highlight_index.0.saturating_sub(self.context_index);
            let offset_disp = extra.line[0..offset].width_cjk();
            for _ in 0..offset_disp {
                buffer.consume(" ");
            }

            let len = extra.line.width_cjk();
            let arrow_close = (self.highlight_index.1 - self.context_index - offset).min(len);
            let arrow_disp = extra.line[offset..arrow_close].width_cjk();
            for _ in 0..arrow_disp {
                buffer.consume("^");
            }
        };
        "\n";

        USIZE_BASE_10_MAX_DIGITS as usize => buffer.consume(&PADDING[0..extra.row_number_max_len as usize]);
        " = ";
        extra.message.len() => buffer.consume(extra.message);
        "\n";

    });
}

/****************************************************************************
 * For printing
 ****************************************************************************/
const USIZE_BASE_10_MAX_DIGITS: usize = count_digits(usize::MAX) as usize;
const PADDING: &str = unsafe { std::str::from_utf8_unchecked(&[b' '; USIZE_BASE_10_MAX_DIGITS]) };

// Although the standard library provides this function (`usize::to_string`)
// I want no-allocations version (mostly as an intellectual exercise)
// similar to https://github.com/rust-lang/rust/blob/1.26.0/src/libcore/fmt/num.rs#L50-L98
fn push_num<U: Consumer>(mut num: usize, buffer: &mut U) {
    let mut temp = [0u8; USIZE_BASE_10_MAX_DIGITS];
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
    buffer.consume(unsafe { std::str::from_utf8_unchecked(&temp[curr..]) });
}

// Count the number of digits in the base-10 representation without allocations
// i.e. Naive alternative is `num.to_string().len()`
const fn count_digits(mut num: usize) -> u8 {
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
