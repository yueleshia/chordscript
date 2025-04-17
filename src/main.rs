// run: cargo test -- --nocapture
//run: cargo run --release

#![allow(dead_code)]
#![allow(clippy::string_lit_as_bytes)]

mod constants;
mod deserialise;
mod errors;
mod keyspace;
mod lexer;
mod macros;
mod parser;
mod reporter;
mod structs;

use deserialise::Print;
use std::fs;

fn main() {
    let file = fs::read_to_string("../../config.txt").unwrap();
    let lexemes = lexer::process(file.as_str()).unwrap();
    let parsemes = parser::process(&lexemes).unwrap();

    let buffer = &mut String::new();
    //deserialise::ListPreview(&parsemes).push_string_into(buffer);
    //deserialise::Shellscript(&parsemes).push_string_into(buffer);

    let _keyspaces = keyspace::process(&parsemes);
    //deserialise::KeyspacePreview(&_keyspaces).push_string_into(buffer);
    //deserialise::I3(&_keyspaces).push_string_into(buffer);
    deserialise::I3Shell(&_keyspaces).push_string_into(buffer);
    println!("{}", buffer);
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
#|| echo asdf
#|super;| echo yo
#|| echo yo
#|super shift q; t|echo {{3349\, 109324}}
|super shift q|"#;

    let _file = r#"
    #
#hello
|super {{, alt, ctrl, ctrl alt}} Return|
  {{$TERMINAL, alacritty, \
  st, sakura}} -e tmux.sh open
|super {{c, t, g, k}} ; super {{b,s}}|
  $TERMINAL -e {{curl,browser.sh}}  '{{terminal,gui}}' '{{bookmarks,search}}'
{{{| cat -}}}jam
|super shift q|"#;
    //println!("{}", _file);

    if let Err(err) = (|| -> Result<(), reporter::MarkupError> {
        let _lexemes = lexer::process(_file)?;
        //_lexemes.to_iter().for_each(print_lexeme);
        //_lexemes.to_iter().for_each(debug_print_lexeme);

        let parsemes = parser::process(&_lexemes)?;
        //println!("{}", deserialise::ListPreview(&parsemes).to_string_custom());
        let keyspaces = keyspace::process(&parsemes);
        //println!("{}", deserialise::KeyspacePreview(&keyspaces).to_string_custom());
        println!("{}", deserialise::I3(&keyspaces).to_string_custom());
        Ok(())
    })() {
        println!("{}", err);
    }
}
