use anyhow::Result;

use crate::cli::args::ConfigCommand;
use crate::{config, output};

pub fn run(command: ConfigCommand) -> Result<()> {
    match command {
        ConfigCommand::Show { json } => {
            let mut loaded = config::load()?;
            loaded.config.ai.base_url =
                crate::security::redact_sensitive(&loaded.config.ai.base_url);
            loaded.config.ai.model = crate::security::redact_sensitive(&loaded.config.ai.model);
            if json {
                println!("{}", serde_json::to_string_pretty(&loaded.config)?);
            } else {
                output::print_config(&loaded);
            }
        }
        ConfigCommand::Set { key, value } => {
            println!(
                "Proposed configuration change\n  {}\n",
                crate::security::redact_sensitive(&format!("{key} = {value}"))
            );
            let path = config::set_value(&key, &value)?;
            println!("✓ Set {key}\n\nConfig file\n  {}", path.display());
        }
    }
    Ok(())
}
