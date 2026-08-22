use anyhow::{Result, bail};

use crate::cli::args::DoctorArgs;

pub fn run(_args: DoctorArgs) -> Result<()> {
    bail!("environment checks are not available yet")
}
