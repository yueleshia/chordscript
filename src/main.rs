// run: cargo test -- --nocapture
// run: cargo run --release

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
use std::io::{self, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match parse_args(&args) {
        Ok(()) => std::process::exit(0),
        Err(Errors::Cli(err)) => eprintln!("{}", err.to_string()),
        Err(Errors::Io(err)) => eprintln!("{}", err.to_string()),
        Err(Errors::Parse(err)) => eprintln!("{}", err.to_string_custom()),
    }
    std::process::exit(1);
}

const DESCRIPTION: &str = "\
Hello";

fn add(opts: &mut getopts::Options, is_required: bool, a: &str, b: &str, c: &str, d: &str) {
    if is_required {
        opts.reqopt(a, b, c, d);
    } else {
        opts.optopt(a, b, c, d);
    }
}

fn options(need_config: bool, need_script: bool) -> getopts::Options {
    let mut opts = getopts::Options::new();
    opts.optflag("h", "help", "print this help menu");
    add(
        &mut opts,
        need_script,
        "s",
        "script",
        "File to output a shellscript",
        "FILENAME",
    );
    add(
        &mut opts,
        need_config,
        "c",
        "config",
        "The config file that specifies hotkeys are we want to compile",
        "FILENAME",
    );
    opts
}

enum Errors {
    Cli(getopts::Fail),
    Io(io::Error),
    Parse(reporter::MarkupError),
}
//run: cargo run -- keyspaces --config $HOME/interim/hk/config.txt #-s $HOME/interim/hk/script.sh
fn parse_args(args: &[String]) -> Result<(), Errors> {
    let program = &args[0];
    let args = &args[1..];
    {
        let opts = options(false, false);
        if opts.parse(args).map_err(Errors::Cli)?.opt_present("h") {
            println!("{}\n{}", program, opts.usage(DESCRIPTION));
            return Ok(());
        }
    }

    let pargs = options(true, false).parse(args).map_err(Errors::Cli)?;

    let file = fs::read_to_string(pargs.opt_str("c").unwrap()).map_err(Errors::Io)?;
    let lexemes = lexer::process(file.as_str()).map_err(Errors::Parse)?;
    let parsemes = parser::process(&lexemes).map_err(Errors::Parse)?;

    match pargs.free.get(0).map(String::as_str) {
        Some("i3") => {
            let pargs = options(true, true).parse(args).map_err(Errors::Cli)?;
            let script_pathstr = pargs.opt_str("s").unwrap();

            let shell = deserialise::Shellscript(&parsemes).to_string_custom();
            let mut script_file = fs::File::create(script_pathstr).map_err(Errors::Io)?;
            script_file
                .write_all(shell.as_bytes())
                .map_err(Errors::Io)?;

            let keyspaces = keyspace::process(&parsemes);
            let i3_config = deserialise::I3Shell(&keyspaces);
            let mut buffer = String::with_capacity(i3_config.string_len());
            i3_config.push_string_into(&mut buffer);
            println!("{}", buffer);
        }
        Some("list") => println!("{}", deserialise::ListPreview(&parsemes).to_string_custom()),
        Some("keyspaces") => println!(
            "{}",
            deserialise::KeyspacePreview(&keyspace::process(&parsemes)).to_string_custom()
        ),

        Some("sh") => println!("{}", deserialise::Shellscript(&parsemes).to_string_custom()),
        x => panic!("Invalid command {:?}", x),
    }
    Ok(())
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
        println!("{}", deserialise::ListPreview(&parsemes).to_string_custom());
        let keyspaces = keyspace::process(&parsemes);
        //println!("{}", deserialise::KeyspacePreview(&keyspaces).to_string_custom());
        //println!("{}", deserialise::I3(&keyspaces).to_string_custom());
        Ok(())
    })() {
        println!("{}", err);
    }
}
