pub mod cli;
pub mod config;
pub mod output;
pub mod scanner;

use clap::Parser;

pub fn run() -> anyhow::Result<()> {
    let cli = cli::args::Cli::parse();
    cli::dispatch::dispatch(cli)
}
