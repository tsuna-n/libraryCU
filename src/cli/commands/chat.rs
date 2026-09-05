use std::io::{self, BufRead, Read, Write};

use anyhow::Result;

use crate::{answer, cli::args::ChatArgs, config, history};

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
    println!("libraryCube chat — /clear clears this session; /exit ends it");
    loop {
        print!("> ");
        io::stdout().flush()?;
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
        if args.ai
            && let Err(error) = answer::enhance(&mut report, &loaded.config.ai, &session_history)
        {
            eprintln!("! AI unavailable: {error:#}; showing the offline answer");
        }
        println!(
            "{}",
            report
                .ai
                .as_ref()
                .map(|ai| ai.analysis.as_str())
                .unwrap_or(&report.offline_answer)
        );
        for warning in &report.warnings {
            eprintln!("! Invalid knowledge document: {warning}");
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
