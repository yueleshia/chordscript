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
  {{$TERMINAL, alacritty, \\
  st, sakura}} -e tmux.sh open
|super {{c, t,g, k, v}} ; super {{b,s}}|
  $TERMINAL -e {{curl,browser.sh}}  '{{terminal,gui}}' '{{bookmarks,search}}'
{{{| cat -}}}
|super shift q|"#;
    //println!("{}", _file);

    let _error = reporter::MarkupError::new(_file, &_file[20..35], "what a failure".to_string());
    let _lexemes = lexer::process(_file).unwrap();
    //println!("{}", _error);

    //for (i, x) in _lexemes.heads.iter().enumerate() {
    //    println!("{}: {:?}", i, x);
    //}
    for (i, x) in _lexemes.bodys.iter().enumerate() {
        println!("{}: {:?}", i, x);
    }
}
