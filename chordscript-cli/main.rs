use std::fs;
use chordscript::deserialise::Print;
use chordscript::keyspace;

mod flags {
    #![allow(unused)]
    xflags::xflags! {
        /// Emulates sxhkd
        cmd chordscript-cli

        {
            /// Print this help menu
            cmd help {}

            /// Print supported frameworks
            cmd frameworks {}

            cmd shell
                required filepath: String
            {}

            /// Use `chordscript-cli shell` to generate the runner. This is referenced by {runner_cmd}
            cmd shellrunner
                required framework: String
                required runner_cmd: String
                required filepath: String
            {}

            cmd native
                required framework: String
                required filepath: String
            {}

        }
    }
}

enum F {
    DebugShortcuts,
    Shell,
    I3,
}

//run: cargo run shellrunner i3 a ~/.config/rc/wm-shortcuts
fn main() {
    match flags::ChordscriptCli::from_env() {
        Ok(args) => match &args.subcommand {
            flags::ChordscriptCliCmd::Help(_) => {
                eprintln!("{}", flags::ChordscriptCli::HELP);
                std::process::exit(1)
            }
            flags::ChordscriptCliCmd::Frameworks(_) => {}

            _ => {
                let (framework_str, framework, runner_cmd, filepath) = match &args.subcommand {
                    flags::ChordscriptCliCmd::Shell(params) => {
                        ("shell", Ok(F::Shell), None, params.filepath.as_str())
                    }
                    flags::ChordscriptCliCmd::Native(params) => {
                        (params.framework.as_str(), Err(params.framework.as_str()), None, params.filepath.as_str())
                    }
                    flags::ChordscriptCliCmd::Shellrunner(params) => (
                        params.framework.as_str(),
                        Err(params.framework.as_str()),
                        Some(params.runner_cmd.as_str()),
                        params.filepath.as_str(),
                    ),

                    // exhausitive listing
                    flags::ChordscriptCliCmd::Help(_) => unreachable!(),
                    flags::ChordscriptCliCmd::Frameworks(_) => unreachable!(),
                };

                let shortcutrc = match fs::read_to_string(filepath) {
                    Ok(a) => a,
                    Err(err) => {
                        eprintln!("Could not read file {:?}\n{}", filepath, err);
                        std::process::exit(1);
                    }
                };

                let lexemes = match chordscript::lexer::lex(&shortcutrc) {
                    Ok(a) => a,
                    Err(err) => {
                        err.print_stderr();
                        std::process::exit(1)
                    }
                };
                let ast = chordscript::lexer::lex(&shortcutrc)
                    .and_then(|lexemes| chordscript::parser::parse_unsorted(lexemes))
                    .unwrap_or_else(|err| {
                        err.print_stderr();
                        std::process::exit(1)
                    });

                let framework = match framework {
                    Ok(a) => a,
                    Err("debug-shortcuts") => F::DebugShortcuts,
                    Err("shell") => F::Shell,
                    Err("i3") => F::I3,
                    Err(f) => {
                        eprintln!("{} is not a suppported framework. Supported frameworks are:\n{}", f, "TODO");
                        std::process::exit(1)
                    }
                };
                match (framework, runner_cmd) {
                    (F::DebugShortcuts, _) => chordscript::deserialise::ListAll(&ast).print_stdout(),
                    (F::Shell, None) => chordscript::deserialise::ListAll(&ast).print_stdout(),
                    //("i3", None) => chordscript::deserialise::I3(&keyspace::process(&ast)).print_stdout(),
                    (F::I3, Some(_)) => chordscript::deserialise::I3Shell(&keyspace::process(&ast)).print_stdout(),
                    //"keyspace" => chordscript::deser
                    (_, None) => {
                        eprintln!("{} must have shell runner.", framework_str);
                        std::process::exit(1)
                    }
                    (_, Some(_)) => {
                        eprintln!("{} does not a shell runner ", framework_str);
                        std::process::exit(1)
                    }
                }
            }
        },
        Err(err) => {
            eprintln!("{}\n{}", err, flags::ChordscriptCli::HELP);
            std::process::exit(1)
        }
    }
}
