use std::io::{self, BufRead, Read, Write};

use anyhow::{Result, bail};

use crate::{
    cli::args::{ExportArgs, ImportArgs},
    knowledge::{bundle, transfer},
    scanner,
};

pub fn export(args: ExportArgs) -> Result<()> {
    let project = scanner::find_project_root(&args.project)?;
    let entries = bundle::collect(&project, args.user)?;
    let output = std::env::current_dir()?.join(&args.file);
    if let Some(directory) = output
        .ancestors()
        .find(|path| path.file_name().is_some_and(|name| name == ".lbc"))
    {
        crate::security::git::ensure_gitignore(
            directory
                .parent()
                .ok_or_else(|| anyhow::anyhow!(".lbc has no project root"))?,
        )?;
    }
    bundle::write(&entries, &output)?;
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"path":output,"entries":entries.len()})
            )?
        );
    } else {
        println!(
            "Exported {} knowledge entries to {}",
            entries.len(),
            crate::security::redact_sensitive(&output.display().to_string())
        );
    }
    Ok(())
}

pub fn import(args: ImportArgs) -> Result<()> {
    let project = scanner::find_project_root(&args.project)?;
    let entries = bundle::read(&args.file)?;
    let plan = transfer::prepare(&project, args.user, entries)?;
    let stderr = io::stderr();
    let mut output = stderr.lock();
    writeln!(
        output,
        "Validated knowledge transfer: {} new, {} conflicting IDs, {} identical duplicates.\nDestination: {}",
        plan.report.added.len(),
        plan.report.conflicts.len(),
        plan.report.duplicates.len(),
        crate::security::redact_sensitive(
            &if args.user {
                crate::knowledge::notes_dir()
            } else {
                project.join(".lbc/knowledge")
            }
            .display()
            .to_string()
        )
    )?;
    for (label, ids) in [
        ("Create", &plan.report.added),
        ("Replace (requires --replace)", &plan.report.conflicts),
        ("Skip identical duplicate", &plan.report.duplicates),
        ("Similar knowledge", &plan.report.similar),
    ] {
        for id in ids {
            writeln!(output, "  {label}: {id}")?;
        }
    }
    if !args.replace && !plan.report.conflicts.is_empty() {
        bail!(
            "conflicting IDs found; review the plan and use --replace to permit replacement; nothing was written"
        );
    }
    let changes = plan.report.added.len() + plan.report.replaced.len();
    if !args.yes && changes > 0 {
        write!(
            output,
            "Apply this plan? Existing conflicting notes will be replaced only with --replace. [y/N]\n> "
        )?;
        output.flush()?;
        let mut confirmation = String::new();
        let bytes = io::stdin().lock().take(257).read_line(&mut confirmation)?;
        if bytes == 0 || bytes > 256 {
            bail!("confirmation ended or was too long; nothing was written");
        }
        match confirmation.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => {}
            "" | "n" | "no" | "cancel" | ":cancel" => {
                writeln!(output, "Cancelled; nothing was written.")?;
                return Ok(());
            }
            _ => bail!("expected yes or no; nothing was written"),
        }
    }
    let report = transfer::apply(plan, args.replace)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "Knowledge transfer complete: {} created, {} replaced, {} identical duplicates skipped.",
            report.added.len(),
            report.replaced.len(),
            report.duplicates.len()
        );
    }
    Ok(())
}
