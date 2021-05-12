use crate::config::Config;
use eyre::Result;
use std::io::{BufRead, ErrorKind};
use std::sync::{Arc, Mutex};

use rink_core::{eval, one_line};

use linefeed::{Interface, ReadResult, Signal};

use crate::fmt::print_fmt;
use crate::RinkHelper;

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
    let rl = Interface::new("rink")?;
    rl.set_prompt("> ")?;

    let ctx = crate::config::load(config)?;
    let ctx = Arc::new(Mutex::new(ctx));
    rl.set_completer(Arc::new(RinkHelper::new(ctx.clone(), config.clone())));
    rl.set_report_signal(Signal::Interrupt, true);
    rl.set_report_signal(Signal::Quit, true);

    let mut hpath = dirs::data_local_dir().map(|mut path| {
        path.push("rink");
        path.push("history.txt");
        path
    });

    if let Some(ref mut path) = hpath {
        if let Err(e) = rl.load_history(&path) {
            if e.kind() == ErrorKind::NotFound {
                println!(
                    "History file {:?} doesn't exist, not loading history.",
                    path
                );
            }
        }
    }

    if let Some(ref mut path) = hpath {
        if let Err(e) = rl.save_history(path) {
            eprintln!("Saving history failed: {}", e);
        }
    }
    loop {
        let res = rl.read_line()?;

        match res {
            ReadResult::Input(line) => {
                let readline: &str = &line;
                match readline {
                    "help" => {
                        println!(
                            "For information on how to use Rink, see the manual: \
                     https://github.com/tiffany352/rink-rs/wiki/Rink-Manual\n\
                     To quit, type `quit`."
                        );
                    }
                    "quit" | ":q" | "exit" => {
                        if let Some(ref mut path) = hpath {
                            if let Err(e) = rl.save_history(path) {
                                eprintln!("Saving history failed: {}", e);
                            }
                        }
                        break;
                    }
                    _ => {
                        match eval(&mut *ctx.lock().unwrap(), &*line) {
                            Ok(v) => {
                                rl.add_history(line);
                                print_fmt(config, &v)
                            }
                            Err(e) => print_fmt(config, &e),
                        };
                        println!();
                    }
                }
            }
            ReadResult::Eof => {
                if let Some(ref mut path) = hpath {
                    if let Err(e) = rl.save_history(path) {
                        eprintln!("Saving history failed: {}", e);
                    }
                }
                break;
            }
            ReadResult::Signal(sig) => {
                if sig == Signal::Interrupt || sig == Signal::Quit {
                    if let Some(ref mut path) = hpath {
                        if let Err(e) = rl.save_history(path) {
                            eprintln!("Saving history failed: {}", e);
                        }
                    }
                    break;
                }
            }
        }
    }
    Ok(())
}
