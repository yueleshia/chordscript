//use crate::constants::AVAILABLE_KEYS;

//run: cargo test -- --nocapture

pub mod lexer {
    use crate::const_concat;

    pub const HEAD_INVALID_CLOSE: &str = "Unexpected bar '|'. Close the enumeration first with '}}'.";
    pub const INVALID_LINE_START: &str = "Valid starting characters for a line are:\n\
        - '#' (comments),\n\
        - '!' (placeholders),\n\
        - '|' (commands)";

    pub const EXCLAIM_IN_HEAD: &str = "You are currently defining a head, not a placeholder.  Did you mean to use '|' instead?";
    pub const BAR_IN_PLACEHOLDER: &str = "You are currently defining a placeholder, not a head.  Did you mean to use '!' instead?";
    pub const HEAD_NO_ESCAPING: &str = "You cannot escape characters with backslash '\\' in the hotkey definition portion.";

    pub const BODY_BRACKET_NO_NEWLINE_BAR: &str = "A '|' here conflicts with starting a new entry.  Close the enumeration first with '}}'.\n\
        If you want a '|' as the first character in line try:\n\
        - '\\n|' on the previous line or\n\
        - '\\|' escaping it on this line.";
    const_concat!(const INVALID_ESCAPE = "This character is not eligible for escaping. You might need to escape a previous '\\'\n\
        Valid escapes are: " => crate::constants::VALID_ESCAPEE_STR);

    //pub const HEAD_COMMA_OUTSIDE_BRACKETS: &str = "Unexpected comma ','. Type 'comma' for the key, ';' for a chord separator. ',' only has meaning inside an enumeration group '{{..}}'.";
    //pub const UNFINISHED_LITERAL: &str = "Missing '}}}' to close the literal text input.";
    //pub const UNFINISHED_BRACKETS: &str = "Missing '}}' to close the permutations bracket.";
    pub const MISSING_LBRACKET: &str =
        "Missing a second opening curly brace. Need '{{' to start an enumeration";
    pub const MISSING_RBRACKET: &str =
        "Missing a second closing curly brace. Need '}}' to close an enumeration";

    pub const MORE_BODY_THAN_HEAD_PERMUTATIONS: &str = "The number of body permutations cannot exceed the number of head permutations.\n\
        Either delete the highlighted body portion or add more options for the head.\n\
        If you want a comma as a text, you escape like '\\,'.";


    pub const END_BEFORE_HEAD_CLOSE: &str = "You did not close the head. Please add a '|'. Alternatively, if you placed '|' intentionally at the start of a line, you may wish to consider the following:\n\
        - '{|\\||}' (literals)\n\
        - '{{\\|}}' (you have to add to each relevant permutation), or\n\
        - '{{|}}' (not necessary to escape the backslash)\n\
        depending on your use case.";
    pub const END_BEFORE_PLACEHOLDER_CLOSE: &str =
        "You did not close the placehoder head. Please add a '!'.";
    pub const END_BEFORE_BRACKET_CLOSE: &str = "\
        Missing a second closing curly brace to close the permutation group. \
        Need '}}' to close. If you want a '}' as output, escape it with backslash \
        like '\\}'.";
}

pub mod parser {
    use crate::const_concat;
    use crate::constants::AVAILABLE_KEYS;

    const_concat!(const INVALID_KEY = "Not a valid key\n" => AVAILABLE_KEYS);
    pub const HOTKEY_DUPLICATE: &str = "This hotkey is defined previously.";
    pub const HOTKEY_UNREACHABLE: &str = "This overall hotkey is not accessible because the part of the hotkey is already defined and will be recognised first.";
}

//pub const PANIC_NON_KEY: &str =
//    "There should only be HeadTypes for chords inside a head choice group";
//pub const PANIC_CHOICE_NON_SECTION: &str =
//    "There should only be BodyType::Section inside a body choice group";
//pub const EMPTY_HOTKEY: &str = "You cannot have an empty hotkey. You can comment this out by prefixing with '#' (This makes it part of the previous command and '#' marks comments in shellscript)";
//pub const PLACEHOLDER_DUPLICATE: &str =
//    "This hotkey is reserved for outer config into which we are embedding these shortcuts.";
//pub const PLACEHOLDER_UNREACHABLE: &str = "This overall hotkey is not accessible because the part of the hotkey is reserved and will be recognised first.";



//pub const PLACEHOLDER_DUPLICATE: &str = "This hotkey is reserved for outer config into which we are embedding these shortcuts.";
//pub const PLACEHOLDER_UNREACHABLE: &str = "This overall hotkey is not accessible because the part of the hotkey is reserved and will be recognised first.";

#[test]
fn const_concat_real_example() {
    use crate::constants::AVAILABLE_KEYS;
    use crate::errors::parser;

    assert_eq!(
        ["Not a valid key\n", AVAILABLE_KEYS].join(""),
        parser::INVALID_KEY
    );
}
