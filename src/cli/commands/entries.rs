use std::{
    io::{self, Read},
    path::Path,
};

use anyhow::{Result, bail};
use serde::Serialize;

use crate::{
    cli::args::{AddArgs, EditArgs, IndexArgs, InspectArgs, ListArgs},
    knowledge,
};

const MAX_INPUT_BYTES: u64 = 256 * 1024;

pub fn add(args: AddArgs) -> Result<()> {
    let body = read_input(args.file.as_deref(), args.stdin)?;
    let entry = knowledge::add_entry(knowledge::AddEntry {
        id: args.id.as_deref(),
        title: &args.title,
        kind: &args.kind,
        body: &body,
        project: args.project.as_deref(),
        overrides: None,
    })?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&entry)?);
    } else {
        println!(
            "Added {}\n  Title: {}\n  Kind: {}\n  Stored at: {}",
            entry.source_id, entry.title, entry.kind, entry.path
        );
    }
    Ok(())
}

pub fn list(args: ListArgs) -> Result<()> {
    let root = crate::scanner::find_project_root(&args.project)?;
    let report = knowledge::load_all_documents(&root)?;
    let documents: Vec<_> = report
        .documents
        .iter()
        .filter(|document| document.effective)
        .collect();
    let entries: Vec<_> = documents
        .iter()
        .map(|document| EntrySummary {
            source_id: &document.source_id,
            id: &document.metadata.id,
            title: &document.title,
            kind: &document.kind,
            source: &document.source,
            source_locator: &document.path,
            verification_status: &document.verification_status,
            overrides: document.metadata.overrides.as_deref(),
        })
        .collect();
    if args.json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else {
        println!("libraryCube entries\n");
        for entry in documents {
            println!(
                "{}\n  {} [{}; {}]\n  {}",
                entry.source_id, entry.title, entry.kind, entry.verification_status, entry.path
            );
            if let Some(target) = &entry.metadata.overrides {
                println!("  Overrides: {target}");
            }
        }
        if !report.invalid.is_empty() {
            eprintln!(
                "! {} invalid knowledge document(s); run `lbc index` for details",
                report.invalid.len()
            );
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct EntrySummary<'a> {
    source_id: &'a str,
    id: &'a str,
    title: &'a str,
    kind: &'a str,
    source: &'a str,
    source_locator: &'a str,
    verification_status: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    overrides: Option<&'a str>,
}

pub fn inspect(args: InspectArgs) -> Result<()> {
    let root = crate::scanner::find_project_root(&args.project)?;
    let entry = knowledge::inspect_entry(&args.source_id, &root)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&entry)?);
    } else {
        println!(
            "{}\nTitle: {}\nKind: {}\nSource: {}\nLocator: {}\nVerification status: {}\nWritable: {}\nEffective: {}\n\n{}",
            entry.source_id,
            entry.title,
            entry.kind,
            entry.source,
            entry.path,
            entry.verification_status,
            entry.writable,
            entry.effective,
            entry.body
        );
    }
    Ok(())
}

pub fn edit(args: EditArgs) -> Result<()> {
    let root = crate::scanner::find_project_root(&args.project)?;
    let replacement = args.file.as_deref().map(read_file).transpose()?;
    let entry = knowledge::edit_entry(knowledge::EditEntry {
        reference: &args.source_id,
        replacement: replacement.as_deref(),
        project: &root,
        create_override: args.r#override,
    })?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&entry)?);
    } else {
        println!("Updated {}\n  {}", entry.source_id, entry.path);
    }
    Ok(())
}

#[derive(Serialize)]
struct IndexReport<'a> {
    status: &'static str,
    documents: usize,
    effective_documents: usize,
    invalid_documents: usize,
    invalid: &'a [knowledge::InvalidDocument],
    persistent_cache: bool,
}

pub fn index(args: IndexArgs) -> Result<()> {
    let root = crate::scanner::find_project_root(&args.project)?;
    let report = knowledge::load_all_documents(&root)?;
    let output = IndexReport {
        status: if report.invalid.is_empty() {
            "valid"
        } else {
            "invalid_documents_found"
        },
        documents: report.documents.len(),
        effective_documents: report
            .documents
            .iter()
            .filter(|document| document.effective)
            .count(),
        invalid_documents: report.invalid.len(),
        invalid: &report.invalid,
        persistent_cache: false,
    };
    if args.json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!(
            "Knowledge index\n  Documents: {}\n  Effective: {}\n  Invalid: {}\n  Retrieval state: rebuilt in memory (no persistent cache)",
            output.documents, output.effective_documents, output.invalid_documents
        );
        for invalid in output.invalid {
            println!("\n! {}\n  {}", invalid.path, invalid.error);
        }
    }
    if report.invalid.is_empty() {
        Ok(())
    } else {
        bail!(
            "knowledge validation found {} invalid document(s)",
            report.invalid.len()
        )
    }
}

fn read_input(file: Option<&Path>, stdin: bool) -> Result<String> {
    if let Some(file) = file {
        return read_file(file);
    }
    if stdin {
        let mut value = String::new();
        io::stdin()
            .lock()
            .take(MAX_INPUT_BYTES + 1)
            .read_to_string(&mut value)?;
        if value.len() as u64 > MAX_INPUT_BYTES {
            bail!("input is larger than 256 KB");
        }
        return Ok(value);
    }
    bail!("provide --file or --stdin")
}
fn read_file(path: &Path) -> Result<String> {
    crate::security::files::read_text(path, MAX_INPUT_BYTES)
}
