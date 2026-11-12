pub mod ai;
pub mod answer;
pub mod cli;
pub mod config;
pub mod diagnostics;
pub mod fixer;
pub mod history;
pub mod knowledge;
pub mod output;
pub mod scanner;
pub mod security;

use clap::{CommandFactory, FromArgMatches};

pub fn run() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    let no_color =
        args.iter().any(|arg| arg == "--no-color") || std::env::var_os("NO_COLOR").is_some();
    let command = cli::args::Cli::command()
        .bin_name("lbc")
        .color(if no_color {
            clap::ColorChoice::Never
        } else {
            clap::ColorChoice::Auto
        });
    let parsed = command
        .try_get_matches_from(args)
        .and_then(|matches| cli::args::Cli::from_arg_matches(&matches));
    let cli = match parsed {
        Ok(cli) => cli,
        Err(error) if !error.use_stderr() => {
            error.print()?;
            return Ok(());
        }
        Err(error) => {
            eprintln!(
                "{}",
                cli::errors::format_usage_error(&error, &cli::errors::language())
            );
            std::process::exit(error.exit_code());
        }
    };
    cli::dispatch::dispatch(cli)
}
