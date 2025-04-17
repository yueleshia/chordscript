//run: cargo test -- --nocapture

#![allow(dead_code)]
#![allow(clippy::string_lit_as_bytes)]

mod constants;
mod errors;
mod keyspace;
mod lexer;
mod macros;
mod parser;
mod reporter;
mod structs;

fn main() {
    println!("Hello, world!");
}

#[test]
fn interpret() {
    let _file = r#"
    #
#hello
|super {{, alt, ctrl, ctrl alt}} Return|
  {{$TERMINAL, alacritty, \
  st, sakura}} -e tmux.sh open
|super {{c, t, g, k}} ; super {{b,s}}|
  $TERMINAL -e {{curl,browser.sh}}  '{{terminal,gui}}' '{{bookmarks,search}}'
{{{| cat -}}}
#| | echo asdf  # @TODO this should not be a lexer error
#|super;| echo yo
#|| echo yo
#|super shift q|echo {{{1,2,3}}}
|super shift q|"#;
    //println!("{}", _file);

    if let Err(err) = (|| -> Result<(), reporter::MarkupError> {
        let _lexemes = lexer::process(_file)?;
        //_lexemes.to_iter().for_each(print_lexeme);
        //_lexemes.to_iter().for_each(debug_print_lexeme);

        let parsemes = parser::process(&_lexemes)?;
        let mut _hotkeys = parsemes.make_owned_view();
        //_hotkeys.sort_unstable_by(|a, b| a.hotkey.cmp(b.hotkey));
        //_hotkeys.iter().for_each(|shortcut| println!("{}", shortcut));
        let _keyspaces = keyspace::process(&parsemes)?;
        keyspace::debug_print_keyspace_owner(&_keyspaces);
        Ok(())
    })() {
        println!("{}", err);
    }
}

fn debug_print_lexeme(lexeme: lexer::Lexeme) {
    let head = lexeme
        .head
        .iter()
        .map(|x| format!("{:?}", x))
        .collect::<Vec<_>>()
        .join(" ");
    let body = lexeme
        .body
        .iter()
        .map(|x| format!("{:?}", x))
        .collect::<Vec<_>>()
        .join(" ");
    print!("|{}|\n  {}\n\n", head, body);
}
fn print_lexeme(lexeme: lexer::Lexeme) {
    use constants::{KEYCODES, MODIFIERS};
    use lexer::BodyType;
    use lexer::HeadType;
    use structs::WithSpan;

    let head = lexeme
        .head
        .iter()
        .filter_map(|head_lexeme| match head_lexeme {
            WithSpan(HeadType::Mod(k), _, _) => Some(MODIFIERS[*k]),
            WithSpan(HeadType::Key(k), _, _) => Some(KEYCODES[*k]),
            WithSpan(HeadType::ChoiceBegin, _, _) => Some("{{"),
            WithSpan(HeadType::ChoiceDelim, _, _) => Some(","),
            WithSpan(HeadType::ChoiceClose, _, _) => Some("}}"),
            WithSpan(HeadType::ChordDelim, _, _) => Some(";"),
            WithSpan(HeadType::Blank, _, _) => None,
        })
        .collect::<Vec<_>>()
        .join(" ");

    let body = lexeme
        .body
        .iter()
        .map(|body_lexeme| match body_lexeme {
            WithSpan(BodyType::Section, _, _) => body_lexeme
                .as_str()
                .lines()
                .map(|line| format!("{:?}", line))
                .collect::<Vec<_>>()
                .join("\n  "),
            WithSpan(BodyType::ChoiceBegin, _, _) => "\n  {{\n    ".to_string(),
            WithSpan(BodyType::ChoiceDelim, _, _) => ",\n    ".to_string(),
            WithSpan(BodyType::ChoiceClose, _, _) => ",\n  }}\n  ".to_string(),
        })
        .collect::<Vec<_>>()
        .join("");
    print!("|{}|\n  {}\n\n", head, body.trim());
}
