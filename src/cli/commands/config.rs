use anyhow::Result;

use crate::cli::args::ConfigCommand;
use crate::{config, output};

pub fn run(command: ConfigCommand) -> Result<()> {
    match command {
        ConfigCommand::Show { json } => {
            let loaded = config::load()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&loaded.config)?);
            } else {
                output::print_config(&loaded);
            }
        }
        ConfigCommand::Set { key, value } => {
            let path = config::set_value(&key, &value)?;
            println!("✓ Set {key}\n\nConfig file\n  {}", path.display());
        }
    }
    Ok(())
}
