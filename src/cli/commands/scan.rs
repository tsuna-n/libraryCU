use anyhow::Result;

use crate::cli::args::ScanArgs;
use crate::{config, output, scanner};

pub fn run(args: ScanArgs) -> Result<()> {
    let loaded = config::load()?;
    let report = scanner::scan_project(&args.path, &loaded.config.scanner)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        output::print_scan(&report, args.tree);
    }
    Ok(())
}
