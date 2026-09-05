use anyhow::Result;

use crate::cli::args::SearchArgs;
use crate::{answer, config, knowledge, output, scanner};

pub fn run(args: SearchArgs) -> Result<()> {
    let root = scanner::find_project_root(&args.project)?;
    let loaded_config = config::load()?;
    let language = answer::choose_language(&loaded_config.config.output.language, &args.query);
    let retrieved = knowledge::retrieve(&root, &args.query)?;
    let results = retrieved.results;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        output::print_search_language(&args.query, &results, &language);
    }
    for invalid in retrieved.invalid {
        eprintln!(
            "! Invalid knowledge document {}: {}",
            invalid.path, invalid.error
        );
    }
    Ok(())
}
