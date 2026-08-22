use anyhow::{Result, bail};

use crate::cli::args::ConfigCommand;

pub fn run(_command: ConfigCommand) -> Result<()> {
    bail!("configuration management is not available yet")
}
