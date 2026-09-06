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
            let message = if report.language == "th" {
                format!("ไม่สามารถใช้ AI ได้: {error:#}")
            } else {
                format!("AI unavailable: {error:#}")
            };
            report.ai_error = Some(message.clone());
            if report.language == "th" {
                eprintln!("! {message}; กำลังแสดงคำตอบแบบออฟไลน์");
            } else {
                eprintln!("! {message}; showing the offline answer");
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
                    is_thinking = true;
                    eprint!(
                        "{}",
                        if thai {
                            "กำลังคิด..."
                        } else {
                            "Thinking..."
                        }
                    );
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
                eprint!("\r\x1b[2K");
                let _ = io::stderr().flush();
            }
            let message = if report.language == "th" {
                format!("ไม่สามารถใช้ AI ได้: {error:#}")
            } else {
                format!("AI unavailable: {error:#}")
            };
            report.ai_error = Some(message.clone());
            if report.language == "th" {
                eprintln!("! {message}; กำลังแสดงคำตอบแบบออฟไลน์");
                println!("\nคำตอบจาก libraryCube\n\n{}", report.offline_answer);
            } else {
                eprintln!("! {message}; showing the offline answer");
                println!("\nlibraryCube answer\n\n{}", report.offline_answer);
            }
        } else if has_content {
            println!();
        }
    }

    for warning in report.warnings {
        if report.language == "th" {
            eprintln!("! เอกสารความรู้ไม่ถูกต้อง: {warning}");
        } else {
            eprintln!("! Invalid knowledge document: {warning}");
        }
    }
    Ok(())
}
