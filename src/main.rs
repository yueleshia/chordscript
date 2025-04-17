//run: cargo test -- --nocapture

#![allow(dead_code)]
mod errors;

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

    let error = errors::MarkupLineError::new("what a failure", _file, 3, 2, 5);
    println!("{}", error);
}
