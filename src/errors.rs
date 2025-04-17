//run: cargo test -- --nocapture

use std::{error, io, fmt, mem};

#[derive(Debug)]
pub enum CliError {
    Markup(MarkupLineError),
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

#[derive(Debug)]
pub struct MarkupLineError {
    source: String,
    row: usize,
    col: (usize, usize),
    message: String,
}
impl MarkupLineError {
    pub fn new(message: &str, source: &str, row: usize, col_from: usize, col_till: usize) -> Self {
        Self {
            source: source.to_string(),
            row,
            col: (col_from, col_till),
            message: message.to_string(),
        }
    }

}
impl error::Error for MarkupLineError {}

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

impl fmt::Display for MarkupLineError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self { source, row, col, message } = self;
        let col = if col.0 < col.1 {
            (col.0, col.1)
        } else {
            (col.1, col.0)
        };
        let line = if let Some(l) = source.lines().nth(*row) {
            l
        } else {
            panic!("The row {} does not exist in the source.\n{}", row, source);
        };
        let col_digits = (count_digits(col.0), count_digits(col.1));
        let row_max_digits = count_digits(*row);
        let mut buffer: String;
        precalculate_capacity_and_build!(buffer, {
            // Print the filename, row, and col span
            row_max_digits => pad(&mut buffer, row_max_digits, "");
            4 => buffer.push_str("--> ");
            8 => buffer.push_str("<source>");
            1 => buffer.push(':');
            row_max_digits => push_num(&mut buffer, row_max_digits, *row);
            1 => buffer.push(':');
            col_digits.0 => push_num(&mut buffer, col_digits.0, col.0);

            if col.0 != col.1 { 1 + col_digits.1 } else { 0 }
            => if col.0 != col.1 {
                buffer.push('-');
                push_num(&mut buffer, col_digits.1, col.1);
            };
            1 => buffer.push('\n');

            // Beginning padding for source code
            row_max_digits => pad(&mut buffer, row_max_digits, "");
            3 => buffer.push_str(" |\n");

            // Print source with row numbers
            row_max_digits => push_num(&mut buffer, row_max_digits, *row);
            3 => buffer.push_str(" | ");
            line.len() => buffer.push_str(line);
            1 => buffer.push('\n');

            // Print error marker
            row_max_digits => pad(&mut buffer, row_max_digits, "");
            3 => buffer.push_str(" | ");
            col.1 => {
                for _ in 0..col.0 {
                    buffer.push(' ');
                }
                for _ in col.0..col.1 {
                    buffer.push('^');
                }
            };
            1 => buffer.push('\n');

            // Closing padding for source code
            row_max_digits => pad(&mut buffer, row_max_digits, "");
            3 => buffer.push_str(" |\n");

            // Print error message
            row_max_digits => pad(&mut buffer, row_max_digits, "");
            3 => buffer.push_str(" = ");
            message.len() => buffer.push_str(message.as_str());
        });

        f.write_str(&buffer)
    }
}

//#[test]
//fn push_num_test() {
//    let mut buffer = String::with_capacity(100);
//    push_num(&mut buffer, 100, usize::MAX);
//    println!("yo |{}|", buffer);
//    println!("yo |{}|", u32::MAX);
//    println!("yo |{}|", mem::size_of::<usize>() * 8 / 2);
//}

// (64 * 3 / 10 + 1 = 20) 20 for 64bit, (32 * 3 / 10 + 1 = 10) 10 for 32bit
// 0.302 is an overestimate of log(10)/log(2) to err on side of bigger
const USIZE_BASE_10_MAX_DIGITS: usize = mem::size_of::<usize>() * 8 * 302 / 1000 + 1;

// Although the standard library provides this function (`usize::to_string`)
// I want no-allocations version (mostly as an intellectual exercise)
// similar to https://github.com/rust-lang/rust/blob/1.26.0/src/libcore/fmt/num.rs#L50-L98
fn push_num(buffer: &mut String, digit_len: usize, mut num: usize) {
    let mut temp: [u8; USIZE_BASE_10_MAX_DIGITS] =
        unsafe { mem::MaybeUninit::uninit().assume_init() };
    let mut curr = temp.len();
    let base = 10;
    loop { // do-while num != 0
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
    return size;
}

fn pad(buffer: &mut String, len: usize, to_pad: &str) {
    assert!(to_pad.len() <= len);
    buffer.push_str(to_pad);
    for _ in 0..len - to_pad.len() {
        buffer.push(' ');
    }
}
