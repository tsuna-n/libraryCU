use anyhow::{Result, bail};

use crate::cli::args::ScanArgs;

pub fn run(_args: ScanArgs) -> Result<()> {
    bail!("project scanning is not available yet")
}
