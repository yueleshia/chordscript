//run: cargo test -- --nocapture

#![allow(dead_code)]
#![allow(clippy::string_lit_as_bytes)]

mod constants;
mod errors;
mod lexer;
mod macros;
mod reporter;

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
|super {{c, t,g, k, v}} ; super {{b,s}}|
  $TERMINAL -e {{curl,browser.sh}}  '{{terminal,gui}}' '{{bookmarks,search}}'
{{{| cat -}}}
|super shift q|"#;
    //println!("{}", _file);

    let _error = reporter::MarkupError::new(_file, &_file[20..35], "what a failure".to_string());
    let _lexemes = lexer::process(_file).unwrap();
    //println!("{}", _error);

    //for (i, x) in _lexemes.head.iter().enumerate() {
    //    println!("{}: {:?}", i, x);
    //}
    //for (i, x) in _lexemes.body.iter().enumerate() {
    //    println!("{}: {:?}", i, x);
    //}
    for entry in  _lexemes.to_iter() {
        use lexer::HeadLexeme;
        use lexer::BodyLexeme;
        use constants::{MODIFIERS, KEYCODES};

        let head = entry.head.iter()
            .filter_map(|head_lexeme| match head_lexeme {
                HeadLexeme::Mod(k) => Some(MODIFIERS[*k]),
                HeadLexeme::Key(k) => Some(KEYCODES[*k]),
                HeadLexeme::ChoiceBegin => Some("{{"),
                HeadLexeme::ChoiceDelim => Some(","),
                HeadLexeme::ChoiceClose => Some("}}"),
                HeadLexeme::ChordDelim => Some(";"),
                HeadLexeme::Blank => None,
            })
            .collect::<Vec<_>>()
            .join(" ");

        let body = entry.body
            .iter()
            .map(|body_lexeme| match body_lexeme {
                BodyLexeme::Section(s) => s.lines()
                    .map(|line| format!("{:?}", line))
                    .collect::<Vec<_>>()
                    .join("\n  "),
                BodyLexeme::ChoiceBegin => "\n  {{\n    ".to_string(),
                BodyLexeme::ChoiceDelim => ",\n    ".to_string(),
                BodyLexeme::ChoiceClose => ",\n  }}\n  ".to_string(),
            })
            .collect::<Vec<_>>()
            .join("");
        print!("|{}|\n  {}\n\n", head, body.trim());
    }
}
