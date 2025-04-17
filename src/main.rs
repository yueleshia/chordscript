//run: cargo test -- --nocapture

#![allow(dead_code)]
mod errors;
mod lexer;
mod messages;
mod constants;

const DEV_PRINT: bool = false;

fn main() {
    println!("Hello, world!");
}

#[test]
fn interpret() {
    let _file = r#"
    #
#hello
|super {{, alt, ctrl, ctrl alt}} Return|
  {{$TERMINAL, alacritty, st, sakura}} -e tmux.sh open
|super {{c, t,g, k, v}} ; super {{b,s}}|
  $TERMINAL -e {{curl,browser.sh}}  '{{terminal,gui}}' '{{bookmarks,search}}'

|super shift q|"#;
    //println!("{}", _file);

    let _error = errors::MarkupLineError::new("what a failure", _file, 3, 2, 5);
    let lexemes = lexer::process(_file).unwrap();
    //println!("{}", error);

    for (i, x) in lexemes.heads.iter().enumerate() {
        println!("{}: {:?}", i, x);
    }
    //println!("{:#?}", tokens.bodys);


}
