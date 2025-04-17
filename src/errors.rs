use crate::const_concat;
use crate::constants::AVAILABLE_KEYS;

//run: cargo test -- --nocapture

pub const HEAD_INVALID_CLOSE: &str = "Unexpected bar '|'. Close the enumeration first with '}}'";
pub const HEAD_NO_ESCAPING: &str = "You cannot escape characters with backslash '\\' in the hotkey definition portion";
pub const HEAD_COMMA_OUTSIDE_BRACKETS: &str = "Unexpected comma ','. Type 'comma' for the key, ';' for a chord separator. ',' only has meaning inside an enumeration group '{{..}}'";
pub const MISSING_LBRACKET: &str = "Missing a second opening curly brace. Need '{{' to start an enumeration";
pub const MISSING_RBRACKET: &str = "Missing a second closing curly brace. Need '}}' to close an enumeration";
const_concat!(const HEAD_INVALID_KEY = "Not a valid key\n" => AVAILABLE_KEYS);


#[test]
fn const_concat_real_example() {
    assert_eq!(["Not a valid key\n", AVAILABLE_KEYS].join(""), HEAD_INVALID_KEY);
}
