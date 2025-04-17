use std::fs;
use chordscript::templates::{Templates, ID_TO_STR, ID_TO_TEMPLATE};
use chordscript::parser::parse_to_shortcuts;

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

#[derive(Debug)]
enum Runner<'a> {
    Shell {
        format: &'static Templates,
        runner: &'a str,
    },
    Native(&'static Templates),
}

fn template_from_str(format: &str) -> Result<&'static Templates, &str> {
    ID_TO_STR.iter().position(|s| *s == format)
        .map(|id| &ID_TO_TEMPLATE[id])
        .ok_or(format)
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
                let (runner, filepath) = match &args.subcommand {
                    flags::ChordscriptCliCmd::Shell(params) => {
                        (Ok(Runner::Native(&ID_TO_TEMPLATE[Templates::ShellScript.id()])), params.filepath.as_str())
                    }
                    flags::ChordscriptCliCmd::Native(params) => {
                        (template_from_str(&params.framework).map(Runner::Native), params.filepath.as_str())
                    }
                    flags::ChordscriptCliCmd::Shellrunner(params) => (
                        template_from_str(&params.framework).map(|format| Runner::Shell {
                            format,
                            runner: &params.runner_cmd,
                        }),
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

                let ast = parse_to_shortcuts(&shortcutrc).unwrap_or_else(|err| {
                    println!("{:?}", err);
                    std::process::exit(1)
                });

                let lock = &mut std::io::stdout();

                match runner {
                    Ok(Runner::Native(format @ Templates::ShellScript)) => format.pipe(&ast, lock).unwrap(),
                    Ok(Runner::Shell{ format, runner: _ }) => format.pipe(&ast, lock).unwrap(),
                    Ok(a) => todo!("{:?}", a),
                    Err(framework) => {
                        eprintln!("{} is not a suppported framework. Supported frameworks are:\n{}", framework, "TODO");
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
