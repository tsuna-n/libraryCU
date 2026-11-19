use std::io::{self, IsTerminal, Read};

use anyhow::{Context, Result, bail};

use crate::cli::args::ExplainArgs;
use crate::{config, diagnostics, output, security};

const MAX_ERROR_INPUT_BYTES: u64 = 2 * 1024 * 1024;

pub fn run(args: ExplainArgs) -> Result<()> {
    let input = read_error_input(&args)?;
    let input = security::redact_sensitive(&input);
    if input.trim().is_empty() {
        bail!("error input is empty");
    }
    let loaded = config::load()?;
    let language = crate::answer::choose_language(&loaded.config.output.language, &input);
    let scope = if args.project_only {
        crate::knowledge::retrieval::RetrievalScope::Project
    } else {
        crate::knowledge::retrieval::RetrievalScope::All
    };
    let report =
        diagnostics::explain_with_scope(&input, &args.project, &loaded.config.scanner, scope)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        output::print_explanation_language(&report, args.verbose, &language);
    }
    Ok(())
}

pub(super) fn read_error_input(args: &ExplainArgs) -> Result<String> {
    if let Some(text) = &args.text {
        if text.len() as u64 > MAX_ERROR_INPUT_BYTES {
            bail!("selected error text is larger than 2 MB");
        }
        return Ok(text.clone());
    }
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
