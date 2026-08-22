pub mod cli;

use clap::Parser;

pub fn run() -> anyhow::Result<()> {
    let cli = cli::args::Cli::parse();
    cli::dispatch::dispatch(cli)
}
