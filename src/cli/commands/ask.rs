use anyhow::Result;

use crate::{answer, cli::args::AskArgs, config, output};

pub fn run(args: AskArgs) -> Result<()> {
    let loaded = config::load()?;
    let project = crate::scanner::find_project_root(&args.project)?;
    let scope = if args.project_only {
        crate::knowledge::retrieval::RetrievalScope::Project
    } else {
        crate::knowledge::retrieval::RetrievalScope::All
    };
    let report = answer::answer_with_scope(
        &args.question,
        &project,
        &loaded.config.output,
        &loaded.config.scanner,
        scope,
    )?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    if report.language == "th" {
        println!("คำตอบจาก libraryCube\n\n{}", report.offline_answer);
    } else {
        println!("libraryCube answer\n\n{}", report.offline_answer);
    }
    for warning in report.warnings {
        if report.language == "th" {
            output::warning(&format!("เอกสารความรู้ไม่ถูกต้อง: {warning}"));
        } else {
            output::warning(&format!("Invalid knowledge document: {warning}"));
        }
    }
    Ok(())
}
