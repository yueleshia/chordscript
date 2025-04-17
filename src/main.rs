//run: cargo test -- --nocapture
// run: cargo run -- --help
// run: cargo run --release
// run: cargo run -- keyspaces --config $XDG_CONFIG_HOME/rc/wm-shortcuts #-s $HOME/interim/hk/script.sh

//#![allow(dead_code)]
#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::mem_discriminant_non_enum)]
#![feature(or_patterns)]

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
//use std::io::{self, Write};
use std::io;

const LONG_HELP: &str = "\
THIS IS WIP
";


fn main() {
    let raw_args = std::env::args().collect::<Vec<_>>();

    let mut options = getopts::Options::new();
    // Yes, we check for '-h' or '--help' twice
    options.optflag("h", "help", "Display a more detailed help menu");
    options.reqopt(
        "c",
        "config",
        "The file containing the shortcuts",
        "FILENAME",
    );

    // Want to exit on help flag no matter want first
    match handle_help_flag(&raw_args)
        .and_then(|_| options.parse(&raw_args[1..]).map_err(Errors::Cli))
        .and_then(subcommands)
    {
        Ok(()) => std::process::exit(0),
        Err(Errors::ShortHelp) => {
            println!("{}", options.short_usage(&raw_args[0]))
        }
        Err(Errors::Help) => {
            println!("{}\n{}", LONG_HELP, options.usage(&raw_args[0]))
        }
        Err(Errors::Cli(err)) => eprintln!("{}", err.to_string()),
        Err(Errors::Io(err)) => eprintln!("{}", err.to_string()),
        Err(Errors::Parse(err)) => eprintln!("{}", err.to_string_custom()),
        //Err(Errors::Debug(err)) => eprintln!("{}", err),
    }
    std::process::exit(1);
}

/****************************************************************************
 * Stuff
 ****************************************************************************/

enum Errors {
    Help,
    ShortHelp,
    Cli(getopts::Fail),
    Io(io::Error),
    Parse(reporter::MarkupError),
    //Debug(String),
}

fn handle_help_flag(input: &[String]) -> Result<(), Errors> {
    for arg in &input[1..] {
        match arg.as_str() {
            "--" => break,
            "-h" | "--help" => return Err(Errors::Help),
            _ => {}
        }
    }
    if input[1..].is_empty() {
        Err(Errors::Help)
    } else {
        Ok(())
    }
}

fn subcommands(matches: getopts::Matches) -> Result<(), Errors> {
    // Must be macro as need to own 'file' in this namespace
    // But want "i3" etc. recognised before requiring 'file'
    macro_rules! process {
        (let $lexemes:ident = @lex $matches:ident) => {
            let path = $matches.opt_str("c").unwrap();
            let file = fs::read_to_string(path).map_err(Errors::Io)?;
            let $lexemes = lexer::lex(&file).map_err(Errors::Parse)?;
            //let lexemes = lexer::process(file.as_str()).map_err(Errors::Parse)?;
        };
        (let $shortcuts:ident = @parse $matches:ident) => {
            process!(let lex_output = @lex $matches);
            let $shortcuts = parser::parse(lex_output).map_err(Errors::Parse)?;
        };
        (let $keyspace:ident = @keyspace $matches:ident) => {
            process!(let parse_output = @parse $matches);
            //let $keyspaces = keyspace::process(&$shortcuts);
            let $keyspace = keyspace::process(&parse_output);
        };
    }

    match matches.free.get(0).map(String::as_str) {
        Some("i3") => {
            //let script_pathstr = pargs.opt_str("s").unwrap();
            //let shell = deserialise::Shellscript(&shortcuts).to_string_custom();
            //let mut script_file = fs::File::create(script_pathstr).map_err(Errors::Io)?;
            //script_file.write_all(shell.as_bytes()).map_err(Errors::Io)?;

            //let i3_config = deserialise::I3Shell(&keyspaces);
            //let mut buffer = String::with_capacity(i3_config.string_len());
            //i3_config.push_string_into(&mut buffer);
            //println!("{}", buffer);
        }
        Some("shortcuts") | Some("shortcut") | Some("s") => {
            process!(let shortcuts = @parse matches);
            println!("{}", deserialise::ListReal(&shortcuts).to_string_custom());
        }
        Some("keyspaces") | Some("keyspace") | Some("k") => {
            process!(let keyspaces = @keyspace matches);
            println!("{}", deserialise::KeyspacePreview(&keyspaces).to_string_custom());
        }
        Some("sh") => {
            //println!("{}", deserialise::Shellscript(&shortcuts).to_string_custom());
        }

        Some("debug-shortcuts") | None => {
            process!(let lexemes = @lex matches);
            //lexemes.lexemes.iter().for_each(|l| println!("{:?}", l));
            let shortcuts = parser::parse_unsorted(lexemes).map_err(Errors::Parse)?;
            println!("{}", deserialise::ListAll(&shortcuts).to_string_custom());
        }

        Some(_) => return Err(Errors::ShortHelp),

    }
    Ok(())
}

/****************************************************************************
 * Integration Tests
 ****************************************************************************/
#[test]
fn on_file() {
    let path = concat!(env!("XDG_CONFIG_HOME"), "/rc/wm-shortcuts");
    let file = fs::read_to_string(path).unwrap();
    let lexemes = lexer::lex(&file).unwrap();
    lexemes.lexemes.iter().for_each(|l| println!("{:?}", l));
    let parsemes = parser::parse(lexemes).unwrap();
    println!("{}", deserialise::ListAll(&parsemes).to_string_custom());
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
        let _lexemes = lexer::lex(_file)?;
        //_lexemes.to_iter().for_each(print_lexeme);
        //_lexemes.to_iter().for_each(debug_print_lexeme);

        let _parsemes = parser::parse(_lexemes)?;
        //println!("{}", deserialise::ListAll(&_parsemes).to_string_custom());
        let _keyspaces = keyspace::process(&_parsemes);
        //println!("{}", deserialise::KeyspacePreview(&keyspaces).to_string_custom());
        //println!("{}", deserialise::I3(&keyspaces).to_string_custom());
        Ok(())
    })() {
        println!("{}", err);
    }
}
