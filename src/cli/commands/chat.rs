use std::io::{self, BufRead, IsTerminal, Read, Write};

use anyhow::Result;

use crate::{ai, answer, cli::args::ChatArgs, config, history, output};

const MAX_HISTORY_MESSAGES: usize = 12;
const MAX_HISTORY_CHARS: usize = 16_000;

pub fn run(args: ChatArgs) -> Result<()> {
    let loaded = config::load()?;
    let project = crate::scanner::find_project_root(&args.project)?;
    let persistent = loaded.config.memory.mode == "persistent";
    let mut session_history: Vec<String> = if persistent {
        history::load()?
    } else {
        Vec::new()
    };
    if !output::is_quiet() && io::stdin().is_terminal() {
        eprintln!("libraryCube chat — /clear clears this session; /exit ends it");
    }
    loop {
        if !output::is_quiet() && io::stdin().is_terminal() {
            eprint!("> ");
            io::stderr().flush()?;
        }
        let mut line = String::new();
        let count = io::stdin()
            .lock()
            .take((crate::knowledge::retrieval::MAX_QUERY_BYTES + 2) as u64)
            .read_line(&mut line)?;
        if count == 0 {
            break;
        }
        if line.trim_end_matches(['\r', '\n']).len() > crate::knowledge::retrieval::MAX_QUERY_BYTES
        {
            anyhow::bail!("chat question exceeds 8 KB; use a shorter question");
        }
        let question = line.trim();
        if question == "/exit" {
            break;
        }
        if question == "/clear" {
            session_history.clear();
            if persistent {
                history::clear()?;
            }
            println!("Session context cleared.");
            continue;
        }
        if question.is_empty() {
            continue;
        }
        let mut report = answer::answer(
            question,
            &project,
            &loaded.config.output,
            &loaded.config.scanner,
        )?;
        let mut is_thinking = false;
        let mut has_streamed = false;
        let thai = report.language == "th";

        let enhance_result = if args.ai {
            let mut on_event = |event: ai::StreamEvent| match event {
                ai::StreamEvent::Thinking => {
                    if !is_thinking && !has_streamed {
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
                    has_streamed = true;
                    print!("{text}");
                    let _ = io::stdout().flush();
                }
            };
            answer::enhance_stream(
                &mut report,
                &loaded.config.ai,
                &session_history,
                Some(&mut on_event),
            )
        } else {
            Ok(())
        };

        if let Err(error) = enhance_result {
            if is_thinking {
                output::clear_progress();
            }
            output::warning(&crate::cli::errors::format_error(&error, &report.language));
            println!("{}", report.offline_answer);
        } else if !has_streamed {
            println!("{}", report.offline_answer);
        } else {
            println!();
        }
        for warning in &report.warnings {
            output::warning(&format!("Invalid knowledge document: {warning}"));
        }
        session_history.push(format!(
            "User: {}",
            crate::security::redact_sensitive(question)
        ));
        session_history.push(format!(
            "Assistant: {}",
            report
                .ai
                .as_ref()
                .map(|ai| ai.analysis.as_str())
                .unwrap_or(&report.offline_answer)
        ));
        while session_history.len() > MAX_HISTORY_MESSAGES
            || session_history.iter().map(String::len).sum::<usize>() > MAX_HISTORY_CHARS
        {
            session_history.remove(0);
        }
        if persistent {
            history::save(&session_history)?;
        }
    }
    Ok(())
}
