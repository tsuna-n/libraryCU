use anyhow::Result;

use crate::cli::args::SearchArgs;
use crate::{answer, config, knowledge, output, scanner};

pub fn run(args: SearchArgs) -> Result<()> {
    let root = scanner::find_project_root(&args.project)?;
    let loaded_config = config::load()?;
    let language = answer::choose_language(&loaded_config.config.output.language, &args.query);
    let retrieved = knowledge::retrieval::retrieve_with_language(&root, &args.query, &language)?;
    let results: Vec<_> = retrieved
        .results
        .into_iter()
        .filter(answer::is_adequate)
        .take(
            args.limit
                .map_or(if args.json { usize::MAX } else { 10 }, |limit| limit.get()),
        )
        .collect();
    if args.json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        output::terminal::print_search_results(
            &crate::security::redact_sensitive(&args.query),
            &results,
            &language,
        );
    }
    for invalid in retrieved.invalid {
        output::warning(&format!(
            "Invalid knowledge document {}: {}; run `lbc index` for details",
            invalid.path, invalid.error
        ));
    }
    Ok(())
}
