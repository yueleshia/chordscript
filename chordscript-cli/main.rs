use std::fs;
use chordscript::{Format, FormatError};
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

            /// This is equivalent to `chordscript-cli native shell`
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
                let (framework, format, filepath) = match &args.subcommand {
                    flags::ChordscriptCliCmd::Shell(params) => {
                        ("shell", Format::from_str("shell", None), params.filepath.as_str())
                    }
                    flags::ChordscriptCliCmd::Native(params) => {
                        let s = params.framework.as_str();
                        (s, Format::from_str(s, None), params.filepath.as_str())
                    }
                    flags::ChordscriptCliCmd::Shellrunner(params) => {
                        let s = params.framework.as_str();
                        (s, Format::from_str(s, Some(&params.runner_cmd)), params.filepath.as_str())
                    }

                    // exhausitive listing
                    flags::ChordscriptCliCmd::Help(_) => unreachable!(),
                    flags::ChordscriptCliCmd::Frameworks(_) => unreachable!(),
                };

                let format = format.map_err(|err| match err {

                    FormatError::Invalid => format!("No filetype named that {:?}", framework),
                    FormatError::NativeUnsupported => format!("Native runner is unsupported. Did you want a shell runner:\n   chordscript shellrunner {}", framework),
                    FormatError::ShellUnsupported => format!("Shell runner is unsupported. Did you want a native runner:\n   chordscript native {}", framework),
                });
                let format = match format {
                    Ok(a) => a,
                    Err(err) => {
                        eprintln!("{}", err);
                        std::process::exit(1);
                    }

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

                format.pipe_stdout(&ast, &mut std::io::stdout());

                //match runner {
                //    Ok(Runner::Native(format @ Templates::ShellScript)
                //        | Runner::Native(format @ Templates::DebugShortcuts)
                //    ) => format.pipe(&ast, lock).unwrap(),
                //    Ok(Runner::Shell{ format, runner: _ }) => format.pipe(&ast, lock).unwrap(),
                //    Ok(a) => todo!("{:?}", a),
                //    Err(framework) => {
                //        eprintln!("{} is not a suppported framework. Supported frameworks are:\n{}", framework, "TODO");
                //        std::process::exit(1)
                //    }
                //}
            }
        },
        Err(err) => {
            eprintln!("{}\n{}", err, flags::ChordscriptCli::HELP);
            std::process::exit(1)
        }
    }
}
