pub mod ai;
pub mod answer;
pub mod cli;
pub mod config;
pub mod diagnostics;
pub mod history;
pub mod knowledge;
pub mod output;
pub mod scanner;
pub mod security;

use clap::Parser;

pub fn run() -> anyhow::Result<()> {
    let cli = cli::args::Cli::parse();
    cli::dispatch::dispatch(cli)
}
