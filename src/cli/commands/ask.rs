use anyhow::Result;

use crate::{answer, cli::args::AskArgs, config};

pub fn run(args: AskArgs) -> Result<()> {
    let loaded = config::load()?;
    let project = crate::scanner::find_project_root(&args.project)?;
    let mut report = answer::answer(
        &args.question,
        &project,
        &loaded.config.output,
        &loaded.config.scanner,
    )?;
    if args.ai
        && let Err(error) = answer::enhance(&mut report, &loaded.config.ai, &[])
    {
        let message = format!("AI unavailable: {error:#}");
        report.ai_error = Some(message.clone());
        eprintln!("! {message}; showing the offline answer");
    }
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("libraryCube answer\n\n{}", report.offline_answer);
        if let Some(ai) = report.ai {
            println!(
                "\nAI analysis ({} / {})\n{}\n\nAI confidence: {}",
                ai.provider, ai.model, ai.analysis, ai.confidence
            );
        }
        if let Some(error) = report.ai_error {
            println!("\nAI status\n{error}");
        }
        for warning in report.warnings {
            eprintln!("! Invalid knowledge document: {warning}");
        }
    }
    Ok(())
}
