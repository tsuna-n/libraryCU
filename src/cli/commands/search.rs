use anyhow::Result;

use crate::cli::args::SearchArgs;
use crate::{knowledge, output, scanner};

pub fn run(args: SearchArgs) -> Result<()> {
    let root = scanner::find_project_root(&args.project)?;
    let documents = knowledge::load_documents(&root)?;
    let results = knowledge::search(&documents, &args.query);
    if args.json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        output::print_search(&args.query, &results);
    }
    Ok(())
}
