use anyhow::Result;

use super::{
    args::{Cli, Command},
    commands,
};

pub fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Add(args) => commands::entries::add(args),
        Command::List(args) => commands::entries::list(args),
        Command::Inspect(args) => commands::entries::inspect(args),
        Command::Edit(args) => commands::entries::edit(args),
        Command::Index(args) => commands::entries::index(args),
        Command::Ask(args) => commands::ask::run(args),
        Command::Chat(args) => commands::chat::run(args),
        Command::History { command } => commands::history::run(command),
        Command::Scan(args) => commands::scan::run(args),
        Command::Explain(args) => commands::explain::run(args),
        Command::Search(args) => commands::search::run(args),
        Command::Config { command } => commands::config::run(command),
        Command::Doctor(args) => commands::doctor::run(args),
        Command::Knowledge { command } => commands::knowledge::run(command),
    }
}
