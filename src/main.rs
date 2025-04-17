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
|super shift q|"#;
    //println!("{}", _file);

    let _error = reporter::MarkupError::new(_file, &_file[20..35], "what a failure".to_string());
    if let Err(err) = (|| -> Result<(), reporter::MarkupError> {
        let _lexemes = lexer::process(_file)?;
        //_lexemes.to_iter().for_each(print_lexeme);
        //_lexemes.to_iter().for_each(debug_print_lexeme);

        let parsemes = parser::process(&_lexemes)?;
        let mut _hotkeys = parsemes.make_owned_view();
        //_hotkeys.sort_unstable_by(|a, b| a.hotkey.cmp(b.hotkey));
        //_hotkeys.iter().for_each(|shortcut| println!("{}", shortcut));
        let _keyspaces = keyspace::process(&parsemes)?;
        Ok(())
    })() {
        println!("{}", err);
    }
    //keyspace::debug_print_keyspace_owner(&_keyspaces);
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
    use lexer::BodyLexeme;
    use lexer::HeadLexeme;
    use structs::WithSpan;

    let head = lexeme
        .head
        .iter()
        .filter_map(|head_lexeme| match head_lexeme {
            WithSpan(HeadLexeme::Mod(k), _, _) => Some(MODIFIERS[*k]),
            WithSpan(HeadLexeme::Key(k), _, _) => Some(KEYCODES[*k]),
            WithSpan(HeadLexeme::ChoiceBegin, _, _) => Some("{{"),
            WithSpan(HeadLexeme::ChoiceDelim, _, _) => Some(","),
            WithSpan(HeadLexeme::ChoiceClose, _, _) => Some("}}"),
            WithSpan(HeadLexeme::ChordDelim, _, _) => Some(";"),
            WithSpan(HeadLexeme::Blank, _, _) => None,
        })
        .collect::<Vec<_>>()
        .join(" ");

    let body = lexeme
        .body
        .iter()
        .map(|body_lexeme| match body_lexeme {
            WithSpan(BodyLexeme::Section, _, _) => body_lexeme
                .as_str()
                .lines()
                .map(|line| format!("{:?}", line))
                .collect::<Vec<_>>()
                .join("\n  "),
            WithSpan(BodyLexeme::ChoiceBegin, _, _) => "\n  {{\n    ".to_string(),
            WithSpan(BodyLexeme::ChoiceDelim, _, _) => ",\n    ".to_string(),
            WithSpan(BodyLexeme::ChoiceClose, _, _) => ",\n  }}\n  ".to_string(),
        })
        .collect::<Vec<_>>()
        .join("");
    print!("|{}|\n  {}\n\n", head, body.trim());
}
