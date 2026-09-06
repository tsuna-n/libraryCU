use std::io::{self, IsTerminal, Read, Write};

use anyhow::{Context, Result, bail};

use crate::cli::args::ExplainArgs;
use crate::{ai, config, diagnostics, output, security};

const MAX_ERROR_INPUT_BYTES: u64 = 2 * 1024 * 1024;

pub fn run(args: ExplainArgs) -> Result<()> {
    let input = read_error_input(&args)?;
    let input = security::redact_sensitive(&input);
    if input.trim().is_empty() {
        bail!("error input is empty");
    }
    let loaded = config::load()?;
    let language = crate::answer::choose_language(&loaded.config.output.language, &input);
    let mut report = diagnostics::explain(&input, &args.project, &loaded.config.scanner)?;

    if args.json {
        if args.ai
            && let Err(error) =
                ai::enhance_with_language(&mut report, &input, &loaded.config.ai, &language)
        {
            report.ai_error = Some(format!("{error:#}"));
        }
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    if !args.ai {
        output::print_explanation_language(&report, args.verbose, &language);
        return Ok(());
    }

    let mut is_thinking = false;
    let mut has_content = false;
    let provider_name = loaded.config.ai.provider.clone();
    let model_name = loaded.config.ai.effective_model().to_owned();

    let mut on_event = |event: ai::StreamEvent| match event {
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
                if language == "th" {
                    println!("\nสิ่งที่ต้องแก้ ({provider_name} / {model_name})");
                } else {
                    println!("\nRequired changes ({provider_name} / {model_name})");
                }
                print!("  ");
                has_content = true;
            }
            let formatted = text.replace('\n', "\n  ");
            print!("{formatted}");
            let _ = io::stdout().flush();
        }
    };

    match ai::enhance_with_language_stream(
        &mut report,
        &input,
        &loaded.config.ai,
        &language,
        Some(&mut on_event),
    ) {
        Ok(()) => {
            if has_content {
                println!();
            }
        }
        Err(error) => {
            if is_thinking {
                eprint!("\r\x1b[2K");
                let _ = io::stderr().flush();
            }
            report.ai_error = Some(format!("{error:#}"));
            eprintln!(
                "! AI enhancement failed; showing the deterministic explanation instead.\n  Reason: {error:#}"
            );
            output::print_explanation_language(&report, args.verbose, &language);
        }
    }

    Ok(())
}

fn read_error_input(args: &ExplainArgs) -> Result<String> {
    if let Some(path) = &args.file {
        return security::files::read_text(path, MAX_ERROR_INPUT_BYTES)
            .with_context(|| format!("failed to read error log {}", path.display()));
    }
    if args.stdin || !io::stdin().is_terminal() {
        let mut input = String::new();
        io::stdin()
            .lock()
            .take(MAX_ERROR_INPUT_BYTES + 1)
            .read_to_string(&mut input)
            .context("failed to read error input from stdin")?;
        if input.len() as u64 > MAX_ERROR_INPUT_BYTES {
            bail!("stdin error input is larger than 2 MB");
        }
        return Ok(input);
    }
    bail!("provide an error log file or pipe error output to `lbc explain`")
}
