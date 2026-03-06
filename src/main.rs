mod cli;
mod commands;
mod completions;
mod config;
mod guide;
mod manifest;
mod plugins;
mod project;
mod remote;
mod scanner;
mod symlinks;

use std::io::ErrorKind;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::CompleteEnv;
use console::Term;

use cli::{Cli, Command};

#[derive(Debug)]
pub(crate) struct UserAbort;

impl std::fmt::Display for UserAbort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("aborted")
    }
}

impl std::error::Error for UserAbort {}

fn main() -> Result<()> {
    CompleteEnv::with_factory(Cli::command).complete();

    ctrlc::set_handler(on_ctrlc)?;

    let cli = Cli::parse();
    let result = match cli.command {
        Command::Tidy(args) => commands::tidy::run(&args),
        Command::Init(args) => commands::init::run(&args),
        Command::Edit => commands::edit::run(),
        Command::Sync => commands::sync::run(),
        Command::Install(args) => commands::install::run(&args),
        Command::Template(args) => commands::template::run(&args.action),
        Command::Guide(args) => commands::guide::run(&args.action),
    };

    if let Err(ref err) = result
        && is_user_abort(err)
    {
        show_cursor();
        return Ok(());
    }

    result
}

fn is_user_abort(err: &anyhow::Error) -> bool {
    if err.downcast_ref::<UserAbort>().is_some() {
        return true;
    }
    err.downcast_ref::<std::io::Error>()
        .is_some_and(|e| matches!(e.kind(), ErrorKind::UnexpectedEof | ErrorKind::Interrupted))
}

fn show_cursor() {
    let term = Term::stderr();
    let _ = term.show_cursor();
}

#[expect(clippy::exit, reason = "signal handler must terminate")]
fn on_ctrlc() {
    show_cursor();
    std::process::exit(130);
}
