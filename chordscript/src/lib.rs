//run: cargo test -- --nocapture

//#![allow(dead_code)]

mod constants;
mod deserialise;
mod errors;
mod macros;
pub mod parser;
mod reporter;
pub mod templates;

pub use crate::parser::{lexemes, shortcuts, keyspaces};

/****************************************************************************
 * Integration Tests
 ****************************************************************************/
#[test]
fn on_file() {
    let path = concat!(env!("XDG_CONFIG_HOME"), "/rc/wm-shortcuts");
    let file = std::fs::read_to_string(path).unwrap();
    let _lexemes = lexemes::lex(&file).unwrap();
    //_lexemes.lexemes.iter().for_each(|l| println!("{:?}", l));
    //println!("\n~~~~~~\n");
    let _shortcuts = shortcuts::parse_unsorted(_lexemes).unwrap();

    // I should not use len() check with externally defined file, but it is
    // the quickest check to see if we altered the algorithm significantly
    println!("~~~~\n{}", _shortcuts.to_iter().count());
    let mut lock = std::io::stdout();
    templates::Templates::DebugShortcuts
        .pipe(&_shortcuts, &mut lock)
        .expect("unreachable");
    println!("~~~~");
    //let _keyspaces = keyspace::process(&_shortcuts);
    //println!("{}", deserialise::KeyspacePreview(&_keyspaces).to_string_custom());
}

#[test]
fn _interpret() {
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
        let _lexemes = lexemes::lex(_file)?;
        //_lexemes.lexemes.iter().for_each(|l| println!("{:?}", l));

        let _parsemes = shortcuts::parse(_lexemes)?;
        //println!("{}", deserialise::ListAll(&_parsemes).to_string_custom());
        let _keyspaces = keyspaces::process(&_parsemes);
        //println!("{}", deserialise::KeyspacePreview(&keyspaces).to_string_custom());
        //println!("{}", deserialise::I3(&keyspaces).to_string_custom());
        Ok(())
    })() {
        use crate::deserialise::Print;
        println!("{}", err.to_string_custom());
    }
}

pub fn add(input: &str) -> String {
    let mut output = input.to_string();
    output.push_str("asdfasdf");
    output
}

//fn subcommands(matches: getopts::Matches) -> Result<(), Errors> {
//    // Must be macro as need to own 'file' in this namespace
//    // But want "i3" etc. recognised before requiring 'file'
//    macro_rules! process {
//        (let $lexemes:ident = @lex $matches:ident) => {
//            let path = $matches.opt_str("c").unwrap();
//            let file = fs::read_to_string(path).map_err(Errors::Io)?;
//            let $lexemes = lexer::lex(&file).map_err(Errors::Parse)?;
//            //let lexemes = lexer::process(file.as_str()).map_err(Errors::Parse)?;
//        };
//        (let $shortcuts:ident = @parse $matches:ident) => {
//            process!(let lex_output = @lex $matches);
//            let $shortcuts = parser::parse(lex_output).map_err(Errors::Parse)?;
//        };
//        (let $keyspace:ident = @keyspace $matches:ident) => {
//            process!(let parse_output = @parse $matches);
//            //let $keyspaces = keyspace::process(&$shortcuts);
//            let $keyspace = keyspace::process(&parse_output);
//        };
//    }
//    macro_rules! build_subcommands_list {
//        ($first_arg:expr => {
//            $($($subcommand:literal)|* => $do:expr,)*
//            @special-case
//            $($rest:tt)*
//        }) => {
//            match $first_arg {
//                $($(Some($subcommand))|* => $do)*
//                Some("subcommands") => {
//                    let list = ["debug-shortcuts", $($($subcommand,)*)*];
//                    println!("{}", list.join("\n"));
//                }
//                $($rest)*
//
//                Some(_) => return Err(Errors::ShortHelp),
//
//            }
//        };
//    }
//    build_subcommands_list!(matches.free.get(0).map(String::as_str) => {
//        "i3-shell" => {
//            let support_path = matches.opt_str("s").expect(
//                "Please specify a -s file for writing the shellscript that \
//                helps i3. We need this because if we included commands \
//                directly in the i3 config, we would need to escape a lot.");
//
//            process!(let shortcuts = @parse matches);
//            let keyspaces = keyspace::process(&shortcuts);
//
//            let i3_config = deserialise::I3Shell(&keyspaces);
//            let mut buffer = String::with_capacity(i3_config.string_len());
//
//            let mut script_file = fs::File::create(support_path).map_err(Errors::Io)?;
//            deserialise::Shellscript(&shortcuts).push_string_into(&mut buffer);
//            script_file.write_all(buffer.as_bytes()).map_err(Errors::Io)?;
//            //println!("{}", buffer);
//
//            buffer.clear();
//            i3_config.push_string_into(&mut buffer);
//            println!("{}", buffer);
//        },
//        "shortcuts" | "shortcut" | "s" => {
//            process!(let shortcuts = @parse matches);
//            deserialise::ListReal(&shortcuts).print_stdout();
//        },
//        "keyspaces" | "keyspace" | "k" => {
//            process!(let keyspaces = @keyspace matches);
//            deserialise::KeyspacePreview(&keyspaces).print_stdout();
//        },
//        "sh" => {
//            process!(let shortcuts = @parse matches);
//            deserialise::Shellscript(&shortcuts).print_stdout();
//        },
//
//        @special-case
//
//        // NOTE: Make sure to update 'list' in the macro
//        Some("debug-shortcuts") | None => {
//            process!(let lexemes = @lex matches);
//            //lexemes.lexemes.iter().for_each(|l| println!("{:?}", l));
//            let shortcuts = parser::parse_unsorted(lexemes).map_err(Errors::Parse)?;
//            deserialise::ListAll(&shortcuts).print_stdout();
//        }
//
//    });
//    Ok(())
//}

