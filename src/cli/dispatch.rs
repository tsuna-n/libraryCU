use anyhow::Result;

use super::{
    args::{Cli, Command},
    commands,
};

pub fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Scan(args) => commands::scan::run(args),
        Command::Explain(args) => commands::explain::run(args),
        Command::Search(args) => commands::search::run(args),
        Command::Config { command } => commands::config::run(command),
        Command::Doctor(args) => commands::doctor::run(args),
        Command::Knowledge { command } => commands::knowledge::run(command),
    }
}
