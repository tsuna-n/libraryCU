use std::io::{self, Write};

use anyhow::Result;

use crate::{ai, answer, cli::args::AskArgs, config, output};

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
            let message = crate::cli::errors::format_error(&error, &report.language);
            report.ai_error = Some(message.clone());
            if report.language == "th" {
                output::warning(&format!("{message}; กำลังแสดงคำตอบแบบออฟไลน์"));
            } else {
                output::warning(&format!("{message}; showing the offline answer"));
            }
        }
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    if !args.ai {
        if report.language == "th" {
            println!("คำตอบจาก libraryCube\n\n{}", report.offline_answer);
        } else {
            println!("libraryCube answer\n\n{}", report.offline_answer);
        }
    } else {
        let mut is_thinking = false;
        let mut has_content = false;
        let thai = report.language == "th";
        let provider_name = loaded.config.ai.provider.clone();
        let model_name = loaded.config.ai.effective_model().to_owned();

        let mut on_event = |event: ai::StreamEvent| match event {
            ai::StreamEvent::Thinking => {
                if !is_thinking && !has_content {
                    is_thinking = output::progress(if thai {
                        "กำลังคิด..."
                    } else {
                        "Thinking..."
                    });
                }
            }
            ai::StreamEvent::Content(text) => {
                if is_thinking {
                    output::clear_progress();
                    is_thinking = false;
                }
                if !has_content {
                    if thai {
                        println!("\nสิ่งที่ต้องแก้ ({provider_name} / {model_name})\n");
                    } else {
                        println!("\nRequired changes ({provider_name} / {model_name})\n");
                    }
                    has_content = true;
                }
                print!("{text}");
                let _ = io::stdout().flush();
            }
        };

        if let Err(error) =
            answer::enhance_stream(&mut report, &loaded.config.ai, &[], Some(&mut on_event))
        {
            if is_thinking {
                output::clear_progress();
            }
            let message = crate::cli::errors::format_error(&error, &report.language);
            report.ai_error = Some(message.clone());
            if report.language == "th" {
                output::warning(&format!("{message}; กำลังแสดงคำตอบแบบออฟไลน์"));
                println!("\nคำตอบจาก libraryCube\n\n{}", report.offline_answer);
            } else {
                output::warning(&format!("{message}; showing the offline answer"));
                println!("\nlibraryCube answer\n\n{}", report.offline_answer);
            }
        } else if has_content {
            println!();
        }
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
