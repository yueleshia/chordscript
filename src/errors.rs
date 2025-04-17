use crate::const_concat;
use crate::constants::AVAILABLE_KEYS;

//run: cargo test -- --nocapture

pub const HEAD_INVALID_CLOSE: &str = "Unexpected bar '|'. Close the enumeration first with '}}'.";
pub const HEAD_NO_ESCAPING: &str =
    "You cannot escape characters with backslash '\\' in the hotkey definition portion.";
pub const HEAD_COMMA_OUTSIDE_BRACKETS: &str = "Unexpected comma ','. Type 'comma' for the key, ';' for a chord separator. ',' only has meaning inside an enumeration group '{{..}}'.";
pub const UNFINISHED_HEAD: &str = "Missing a closing '|' to finish the hotkey definition.";
pub const UNFINISHED_LITERAL: &str = "Missing '}}}' to close the literal text input.";
pub const UNFINISHED_BRACKETS: &str = "Missing '}}' to close the permutations bracket.";
pub const MISSING_LBRACKET: &str =
    "Missing a second opening curly brace. Need '{{' to start an enumeration";
pub const MISSING_RBRACKET: &str =
    "Missing a second closing curly brace. Need '}}' to close an enumeration";
const_concat!(const HEAD_INVALID_KEY = "Not a valid key\n" => AVAILABLE_KEYS);

pub const PANIC_NON_KEY: &str = "There should only be HeadTypes for chords inside a head choice group";
pub const PANIC_CHOICE_NON_SECTION: &str = "There should only be BodyType::Section inside a body choice group";
pub const EMPTY_HOTKEY: &str = "You cannot have an empty hotkey. You can comment this out by prefixing with '#' (This makes it part of the previous command and '#' marks comments in shellscript)";
pub const TOO_MUCH_BODY: &str = "No hotkey is mapped to this permutation. There are too many choices for this command.";
pub const HOTKEY_DUPLICATE: &str = "This hotkey is already defined previously.";
pub const HOTKEY_UNREACHABLE: &str = "The overall hotkey is not accessible because the part of the hotkey is already defined and will be recognised first.";

#[test]
fn const_concat_real_example() {
    assert_eq!(
        ["Not a valid key\n", AVAILABLE_KEYS].join(""),
        HEAD_INVALID_KEY
    );
}
