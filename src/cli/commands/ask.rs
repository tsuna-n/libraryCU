use std::io::{self, Write};

use anyhow::Result;

use crate::{ai, answer, cli::args::AskArgs, config};

pub fn run(args: AskArgs) -> Result<()> {
    let loaded = config::load()?;
    let project = crate::scanner::find_project_root(&args.project)?;
    let mut report = answer::answer(
        &args.question,
        &project,
        &loaded.config.output,
        &loaded.config.scanner,
    )?;
    if args.json {
        if args.ai
            && let Err(error) = answer::enhance(&mut report, &loaded.config.ai, &[])
        {
            let message = format!("AI unavailable: {error:#}");
            report.ai_error = Some(message.clone());
            eprintln!("! {message}; showing the offline answer");
        }
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    println!("libraryCube answer\n\n{}", report.offline_answer);

    if args.ai {
        let mut is_thinking = false;
        let mut has_content = false;
        let provider_name = loaded.config.ai.provider.clone();
        let model_name = loaded.config.ai.effective_model().to_owned();

        let mut on_event = |event: ai::StreamEvent| {
            match event {
                ai::StreamEvent::Thinking => {
                    if !is_thinking && !has_content {
                        is_thinking = true;
                        eprint!("Thinking...");
                        let _ = io::stderr().flush();
                    }
                }
                ai::StreamEvent::Content(text) => {
                    if is_thinking {
                        eprint!("\r\x1b[2K");
                        let _ = io::stderr().flush();
                        is_thinking = false;
                    }
                    if !has_content {
                        println!("\nAI analysis ({provider_name} / {model_name})\n");
                        has_content = true;
                    }
                    print!("{text}");
                    let _ = io::stdout().flush();
                }
            }
        };

        if let Err(error) = answer::enhance_stream(
            &mut report,
            &loaded.config.ai,
            &[],
            Some(&mut on_event),
        ) {
            if is_thinking {
                eprint!("\r\x1b[2K");
                let _ = io::stderr().flush();
            }
            let message = format!("AI unavailable: {error:#}");
            report.ai_error = Some(message.clone());
            eprintln!("! {message}; showing the offline answer");
            println!("\nAI status\n{message}");
        } else if let Some(ai) = &report.ai {
            if has_content {
                println!("\n\nAI confidence: {}", ai.confidence);
            }
        }
    }

    for warning in report.warnings {
        eprintln!("! Invalid knowledge document: {warning}");
    }
    Ok(())
}
