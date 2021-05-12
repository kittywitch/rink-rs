use crate::config::Config;
use eyre::Result;
use rustyline::{config::{Configurer, EditMode, BellStyle}, error::ReadlineError, CompletionType, Editor};
use std::io::{BufRead, ErrorKind};
use std::sync::{Arc, Mutex};

use linefeed::{inputrc,inputrc::Directive};
use rink_core::{eval, one_line};

use crate::fmt::print_fmt;
use crate::RinkHelper;

use std::env;

pub fn noninteractive<T: BufRead>(mut f: T, config: &Config, show_prompt: bool) -> Result<()> {
    use std::io::{stdout, Write};

    let mut ctx = crate::config::load(config)?;
    let mut line = String::new();
    loop {
        if show_prompt {
            print!("> ");
        }
        stdout().flush().unwrap();
        if f.read_line(&mut line).is_err() {
            return Ok(());
        }
        // the underlying file object has hit an EOF if we try to read a
        // line but do not find the newline at the end, so let's break
        // out of the loop
        if line.find('\n').is_none() {
            return Ok(());
        }
        match one_line(&mut ctx, &*line) {
            Ok(v) => println!("{}", v),
            Err(e) => println!("{}", e),
        };
        line.clear();
    }
}

pub fn interactive(config: &Config) -> Result<()> {
    let inputrc_path = env::var("INPUTRC")?;
    let inputrc_config = inputrc::parse_file(&inputrc_path).unwrap_or_default();
    let mut rustyl_config = rustyline::config::Config::builder();

    for directive in inputrc_config {
        match directive {
            Directive::SetVariable(key, value) => match key.as_str() {
                "editing-mode" => rustyl_config = rustyl_config.edit_mode( match value.as_str() {
                    "vi" => EditMode::Vi,
                    "emacs" | _ => EditMode::Emacs,
                }),
                "bell-style" => rustyl_config = rustyl_config.bell_style( match value.as_str() {
                    "none" => BellStyle::None,
                    "visible" => BellStyle::Visible,
                    "audible" | _ => BellStyle::Audible,
                }),
                "keyseq-timeout" => rustyl_config = rustyl_config.keyseq_timeout(value.parse::<i32>()?),
                _ => (),
            },
            _ => (),
        }
    }

    let mut rl = Editor::<RinkHelper>::with_config(rustyl_config.build());

    let ctx = crate::config::load(config)?;
    let ctx = Arc::new(Mutex::new(ctx));
    let helper = RinkHelper::new(ctx.clone(), config.clone());
    rl.set_helper(Some(helper));
    rl.set_completion_type(CompletionType::List);

    let mut hpath = dirs::data_local_dir().map(|mut path| {
        path.push("rink");
        path.push("history.txt");
        path
    });
    if let Some(ref mut path) = hpath {
        match rl.load_history(path) {
            // Ignore file not found errors.
            Err(ReadlineError::Io(ref err)) if err.kind() == ErrorKind::NotFound => (),
            Err(err) => eprintln!("Loading history failed: {}", err),
            _ => (),
        };
    }

    let save_history = |rl: &mut Editor<RinkHelper>| {
        if let Some(ref path) = hpath {
            // ignore error - if this fails, the next line will as well.
            let _ = std::fs::create_dir_all(path.parent().unwrap());
            rl.save_history(path).unwrap_or_else(|e| {
                eprintln!("Saving history failed: {}", e);
            });
        }
    };

    loop {
        let readline = rl.readline(&config.rink.prompt);
        match readline {
            Ok(ref line) if line == "help" => {
                println!(
                    "For information on how to use Rink, see the manual: \
                     https://github.com/tiffany352/rink-rs/wiki/Rink-Manual\n\
                     To quit, type `quit`."
                );
            }
            Ok(ref line) if line == "quit" || line == ":q" || line == "exit" => {
                save_history(&mut rl);
                break;
            }
            Ok(line) => {
                match eval(&mut *ctx.lock().unwrap(), &*line) {
                    Ok(v) => {
                        rl.add_history_entry(line);
                        print_fmt(config, &v)
                    }
                    Err(e) => print_fmt(config, &e),
                };
                println!();
            }
            Err(ReadlineError::Interrupted) => {}
            Err(ReadlineError::Eof) => {
                save_history(&mut rl);
                break;
            }
            Err(err) => {
                println!("Readline: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}
