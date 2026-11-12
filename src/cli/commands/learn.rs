use std::{
    fmt,
    io::{self, BufRead, Read, Write},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Result, bail};

use crate::{cli::args::LearnArgs, knowledge, scanner, security};

const MAX_FIELD_BYTES: usize = 16 * 1024;

#[derive(Debug)]
struct Cancelled;

impl fmt::Display for Cancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Cancelled; nothing was saved.")
    }
}

impl std::error::Error for Cancelled {}

pub fn run(args: LearnArgs) -> Result<()> {
    match capture(args) {
        Err(error) if error.is::<Cancelled>() => {
            eprintln!("{error}");
            Ok(())
        }
        result => result,
    }
}

fn capture(mut args: LearnArgs) -> Result<()> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut prompts = io::stderr().lock();
    if args.yes && args.problem.is_none() && args.from.is_none() {
        bail!("--yes requires --problem or --from; nothing was saved");
    }
    if let Some(path) = &args.from {
        let observed = super::learn_log::read(path)?;
        if observed.problem.is_none() {
            writeln!(
                prompts,
                "No supported diagnostic found; provide the observed problem with --problem or at the prompt."
            )?;
        }
        // Diagnostic messages are single lines; a prefix keeps log text from becoming a Markdown heading/fence.
        args.problem = args.problem.or_else(|| {
            observed
                .problem
                .map(|problem| format!("Observed error: {problem}"))
        });
        args.error_code = args.error_code.or(observed.error_code);
        args.language = args.language.or(observed.language);
        args.tool = args.tool.or(observed.tool);
        args.context = Some(match args.context {
            Some(context) => format!("{context}\n\n{}", observed.context),
            None => observed.context,
        });
    }
    if !args.yes {
        writeln!(
            prompts,
            "Capture a fix that worked. Type :cancel at any prompt to stop."
        )?;
        args.problem = Some(field(
            &mut input,
            &mut prompts,
            args.problem,
            "What happened?",
        )?);
        args.solution = Some(field(
            &mut input,
            &mut prompts,
            args.solution,
            "How was it fixed?",
        )?);
    }
    let lookup_project =
        scanner::find_project_root(args.project.as_deref().unwrap_or(Path::new(".")))?;
    let project = (!args.user).then_some(lookup_project.as_path());
    let (mut title, mut body, metadata) = prepare(&args, project)?;
    let report = knowledge::load_all_documents(&lookup_project)?;
    if !report.invalid.is_empty() {
        writeln!(
            prompts,
            "Warning: {} invalid knowledge entries could not be checked for duplicates.",
            report.invalid.len()
        )?;
    }
    let matches = knowledge::duplicates::find_similar(
        &report.documents,
        &title,
        &clean(args.problem.as_deref().unwrap_or_default())?,
        &metadata,
    );
    let mut editing = None;
    if !matches.is_empty() {
        writeln!(prompts, "\nSimilar knowledge (review before saving):")?;
        for (index, document) in matches.iter().enumerate() {
            writeln!(
                prompts,
                "  {}. {} — {}{}",
                index + 1,
                display(&document.source_id),
                display(&document.title),
                if document.writable {
                    ""
                } else {
                    " (read-only)"
                }
            )?;
        }
        if args.yes {
            if !args.create_new {
                bail!(
                    "similar knowledge exists; run interactive learn to review/edit, or pass --new to explicitly create a new entry; nothing was saved"
                );
            }
        } else {
            let choice = prompt(
                &mut input,
                &mut prompts,
                "Choose new / entry number to replace its body / cancel [cancel]",
            )?;
            match choice.to_ascii_lowercase().as_str() {
                "new" => {}
                "" | "cancel" => return Err(Cancelled.into()),
                _ => {
                    let selected = choice
                        .parse::<usize>()
                        .ok()
                        .and_then(|index| index.checked_sub(1))
                        .and_then(|index| matches.get(index))
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "choose new, a listed entry number, or cancel; nothing was saved"
                            )
                        })?;
                    if !selected.writable {
                        bail!(
                            "{} is read-only; choose new to create a separate entry; nothing was saved",
                            display(&selected.source_id)
                        );
                    }
                    args.title = Some(selected.title.clone());
                    (title, body, _) = prepare(&args, project)?;
                    editing = Some(*selected);
                }
            }
        }
    }
    if !args.yes || args.from.is_some() {
        let preview_metadata = editing.map(|entry| &entry.metadata).unwrap_or(&metadata);
        let status = if let Some(original) = editing {
            writeln!(
                prompts,
                "\nReplacing body of {}. Existing metadata is kept; verification becomes unverified when changed.\nCurrent body:\n{}",
                display(&original.source_id),
                display(&original.body)
            )?;
            "unverified on change"
        } else {
            metadata
                .verification_status
                .as_deref()
                .unwrap_or("unverified")
        };
        writeln!(
            prompts,
            "\nPreview:\n{}\nError code: {}\nLanguage: {}\nTool: {}\nSave to: {}\nVerification status: {status} (not rerun)",
            display(&body),
            display(preview_metadata.error_code.as_deref().unwrap_or("unknown")),
            display(preview_metadata.language.as_deref().unwrap_or("unknown")),
            display(preview_metadata.tool.as_deref().unwrap_or("unknown")),
            display(&editing.map(|entry| entry.path.clone()).unwrap_or_else(|| {
                project
                    .map(|path| path.join(".lbc/knowledge").display().to_string())
                    .unwrap_or_else(|| knowledge::notes_dir().display().to_string())
            })),
        )?;
    }
    if !args.yes {
        let confirm = prompt(&mut input, &mut prompts, "Save this knowledge? [y/N]")?;
        match confirm.to_lowercase().as_str() {
            "y" | "yes" => {}
            "" | "n" | "no" => return Err(Cancelled.into()),
            _ => bail!("expected yes or no; nothing was saved"),
        }
    }
    let entry = if let Some(original) = editing {
        knowledge::storage::edit_entry_from_preview(original, &body, &lookup_project)?
    } else {
        knowledge::storage::add_entry_with_metadata(
            knowledge::AddEntry {
                id: args.id.as_deref(),
                title: &title,
                kind: "troubleshooting",
                body: &body,
                project,
                overrides: None,
            },
            metadata,
        )?
    };
    if args.json {
        println!("{}", serde_json::to_string_pretty(&entry)?);
    } else {
        println!(
            "{} {}\n  {}\n  Verification: {} (recorded only; not rerun)\n  Find it: lbc search {:?}",
            if editing.is_some() {
                "Updated"
            } else {
                "Saved"
            },
            entry.source_id,
            entry.path,
            entry.verification_status,
            entry.metadata.error_code.as_deref().unwrap_or(&title)
        );
    }
    Ok(())
}

fn prompt(input: &mut impl BufRead, output: &mut impl Write, label: &str) -> Result<String> {
    write!(output, "{label}\n> ")?;
    output.flush()?;
    let mut line = String::new();
    let bytes = input
        .take((MAX_FIELD_BYTES + 1) as u64)
        .read_line(&mut line)?;
    if bytes == 0 {
        bail!(
            "capture input ended; nothing was saved (use --yes --problem ... --solution ... for scripts)"
        );
    }
    if bytes > MAX_FIELD_BYTES {
        bail!("capture field exceeds 16 KB; nothing was saved");
    }
    let value = line.trim();
    if value == ":cancel" {
        return Err(Cancelled.into());
    }
    Ok(value.to_owned())
}

fn display(value: &str) -> String {
    security::redact_sensitive(value)
        .chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .collect()
}

fn field(
    input: &mut impl BufRead,
    output: &mut impl Write,
    value: Option<String>,
    label: &str,
) -> Result<String> {
    if let Some(value) = value {
        return Ok(value);
    }
    loop {
        let value = prompt(input, output, label)?;
        if !value.is_empty() {
            return Ok(value);
        }
        writeln!(output, "Please provide some text.")?;
    }
}

fn clean(value: &str) -> Result<String> {
    if value.len() > MAX_FIELD_BYTES {
        bail!("capture field exceeds 16 KB; nothing was saved");
    }
    if value.contains('\0') {
        bail!("capture fields must be text without NUL bytes; nothing was saved");
    }
    Ok(security::redact_sensitive(value.trim()))
}

fn prepare(
    args: &LearnArgs,
    project: Option<&Path>,
) -> Result<(String, String, knowledge::KnowledgeMetadata)> {
    let problem = clean(args.problem.as_deref().unwrap_or_default())?;
    let solution = clean(args.solution.as_deref().unwrap_or_default())?;
    if problem.is_empty() || solution.is_empty() {
        bail!("problem and solution must not be empty; nothing was saved");
    }
    let cause = clean(args.cause.as_deref().unwrap_or_default())?;
    let verification = clean(args.verification.as_deref().unwrap_or_default())?;
    let title = clean(
        args.title
            .as_deref()
            .unwrap_or_else(|| problem.lines().next().unwrap_or(&problem)),
    )?;
    let title: String = title.chars().take(120).collect();
    if title.is_empty() {
        bail!("title must not be empty; nothing was saved");
    }
    let text = format!("{problem}\n{cause}\n{solution}\n{verification}");
    let code = args
        .error_code
        .as_deref()
        .map(clean)
        .transpose()?
        .filter(|s| !s.is_empty())
        .or_else(|| infer_code(&problem));
    let language = args
        .language
        .as_deref()
        .map(clean)
        .transpose()?
        .filter(|s| !s.is_empty())
        .or_else(|| infer_language(&text, code.as_deref()))
        .or_else(|| {
            if !language_hints(&text).is_empty() {
                return None;
            }
            let detected = scanner::detect_project(project?).ok()?;
            if detected.languages.len() == 1 {
                infer_language(&detected.languages[0], None)
            } else {
                None
            }
        });
    let tool = args
        .tool
        .as_deref()
        .map(clean)
        .transpose()?
        .filter(|s| !s.is_empty())
        .or_else(|| {
            text.split_whitespace()
                .find(|word| {
                    matches!(
                        *word,
                        "cargo" | "rustc" | "pytest" | "npm" | "pnpm" | "tsc" | "docker" | "git"
                    )
                })
                .map(str::to_owned)
        });
    let mut tags = args
        .tags
        .iter()
        .map(|s| clean(s))
        .collect::<Result<Vec<_>>>()?;
    if let Some(language) = &language
        && !tags.contains(language)
    {
        tags.push(language.clone());
    }
    if code.as_deref() == Some("E0308") && !tags.iter().any(|tag| tag == "type-mismatch") {
        tags.push("type-mismatch".to_owned());
    }
    tags.retain(|tag| !tag.is_empty());
    tags.sort();
    tags.dedup();
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let metadata = knowledge::KnowledgeMetadata {
        language,
        tool,
        framework: args
            .framework
            .as_deref()
            .map(clean)
            .transpose()?
            .filter(|s| !s.is_empty()),
        error_code: code,
        tags,
        verification_status: Some(
            if verification.is_empty() {
                "unverified"
            } else {
                "recorded-check"
            }
            .to_owned(),
        ),
        created_at_unix: Some(now),
        updated_at_unix: Some(now),
        ..Default::default()
    };
    let mut body = format!("# {title}\n\n## Problem\n\n{problem}\n");
    for (name, value) in [
        ("Symptoms", args.symptoms.as_deref()),
        ("Context", args.context.as_deref()),
        ("Cause", Some(cause.as_str())),
        ("Solution", Some(solution.as_str())),
        ("Verification", Some(verification.as_str())),
    ] {
        let value = clean(value.unwrap_or_default())?;
        if !value.is_empty() {
            body.push_str(&format!("\n## {name}\n\n{value}\n"));
        }
    }
    if !args.references.is_empty() {
        body.push_str("\n## References\n\n");
        for reference in &args.references {
            body.push_str(&format!("- {}\n", clean(reference)?));
        }
    }
    Ok((title, body, metadata))
}

fn infer_code(problem: &str) -> Option<String> {
    problem
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .find(|word| {
            (word.len() == 5
                && word.starts_with('E')
                && word[1..].bytes().all(|b| b.is_ascii_digit()))
                || (word.starts_with("TS")
                    && word.len() > 2
                    && word[2..].bytes().all(|b| b.is_ascii_digit()))
                || word.starts_with("ERR_")
                || matches!(
                    *word,
                    "EADDRINUSE"
                        | "ENOENT"
                        | "ECONNREFUSED"
                        | "ETIMEDOUT"
                        | "ModuleNotFoundError"
                        | "ImportError"
                        | "TypeError"
                        | "ValueError"
                        | "NameError"
                        | "AttributeError"
                )
        })
        .map(str::to_owned)
}

fn infer_language(text: &str, code: Option<&str>) -> Option<String> {
    let found = language_hints(text);
    if found.len() == 1 {
        return Some(found[0].to_owned());
    }
    if !found.is_empty() {
        return None;
    }
    match code {
        Some(code)
            if code.len() == 5
                && code.starts_with('E')
                && code[1..].bytes().all(|b| b.is_ascii_digit()) =>
        {
            Some("rust".to_owned())
        }
        Some(code) if code.starts_with("TS") => Some("typescript".to_owned()),
        Some(code) if code.starts_with("ERR_") => Some("javascript".to_owned()),
        _ => None,
    }
}

fn language_hints(text: &str) -> Vec<&'static str> {
    let words: Vec<_> = text.split(|c: char| !c.is_alphanumeric()).collect();
    let mut found = Vec::new();
    for word in words {
        let value = match word.to_lowercase().as_str() {
            "rust" | "rustc" | "cargo" => "rust",
            "python" | "pytest" => "python",
            "typescript" | "tsc" => "typescript",
            "javascript" | "node" | "nodejs" => "javascript",
            "go" if word == "Go" => "go",
            "java" => "java",
            "kotlin" => "kotlin",
            _ => continue,
        };
        if !found.contains(&value) {
            found.push(value);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inference_is_conservative_and_keeps_identifier_boundaries() {
        assert_eq!(
            infer_code("Rust error[E0308]: mismatch"),
            Some("E0308".into())
        );
        assert_eq!(
            infer_code("Node ERR_MODULE_NOT_FOUND"),
            Some("ERR_MODULE_NOT_FOUND".into())
        );
        assert_eq!(infer_code("MY_E0308_EXTRA E03080"), None);
        assert_eq!(infer_language("Rust Python bridge", None), None);
        assert_eq!(infer_language("go to a directory", None), None);
    }

    #[test]
    fn eof_is_not_confirmation_and_fields_are_bounded() {
        assert!(prompt(&mut io::Cursor::new(""), &mut Vec::new(), "Save?").is_err());
        assert!(
            prompt(
                &mut io::Cursor::new("x".repeat(MAX_FIELD_BYTES + 1)),
                &mut Vec::new(),
                "Problem?"
            )
            .is_err()
        );
        assert_eq!(
            prompt(
                &mut io::Cursor::new("ภาษาไทย\n"),
                &mut Vec::new(),
                "Problem?"
            )
            .unwrap(),
            "ภาษาไทย"
        );
    }
}
