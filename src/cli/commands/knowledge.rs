use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::cli::args::{KnowledgeCommand, KnowledgeValidationArgs};
use crate::knowledge;

pub fn run(command: KnowledgeCommand) -> Result<()> {
    match command {
        KnowledgeCommand::Validate(args) => validate(args, false),
        KnowledgeCommand::Review(args) => validate(args, true),
        KnowledgeCommand::Install { source, bundled } => {
            install(source.as_deref(), bundled.as_deref())
        }
        KnowledgeCommand::List { bundled, json } => list(bundled, json),
        KnowledgeCommand::Remove { name, yes } => remove(&name, yes),
    }
}

fn validate(args: KnowledgeValidationArgs, review: bool) -> Result<()> {
    let root = crate::scanner::find_project_root(&args.project)?;
    let mut report = knowledge::validation::validate(&root, args.max_age_days)?;
    if review {
        report.warnings.retain(|warning| {
            matches!(
                warning.code.as_str(),
                "missing_dates" | "stale_document" | "inconsistent_dates" | "future_dates"
            )
        });
    }
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "Knowledge {}: {} documents, {} warning(s)",
            if review { "review" } else { "validation" },
            report.documents,
            report.warnings.len()
        );
        for warning in report.warnings {
            println!(
                "Warning [{}] {}\n  {}",
                warning.code,
                warning.source_id.as_deref().unwrap_or(&warning.path),
                warning.message
            );
        }
    }
    Ok(())
}

fn install(source: Option<&Path>, bundled: Option<&str>) -> Result<()> {
    let data_dir = knowledge::data_dir();
    let installed = if let Some(name) = bundled {
        knowledge::packages::install_bundled_package(name, &data_dir)?
    } else {
        let source = source.context("provide a local directory or --bundled NAME")?;
        if source.to_string_lossy().contains("://") {
            bail!(
                "remote package sources are unsupported; use a local directory or --bundled NAME; nothing was downloaded"
            );
        }
        knowledge::install_package(source, &data_dir)?
    };
    println!("Installed knowledge package\n");
    println!("  Name:        {}", installed.name);
    println!("  Version:     {}", installed.version);
    println!("  Documents:   {}", installed.documents);
    println!("  Integrity:   {}", installed.integrity);
    println!("  Location:    {}", installed.path.display());
    Ok(())
}

fn list(bundled: bool, json: bool) -> Result<()> {
    if bundled {
        let available = knowledge::packages::bundled_packages();
        if json {
            println!("{}", serde_json::to_string_pretty(&available)?);
        } else {
            println!("Bundled knowledge packages (offline)\n");
            for package in available {
                println!(
                    "  {} {}\n    {}",
                    package.name, package.version, package.description
                );
                println!(
                    "    Install: lbc knowledge install --bundled {}",
                    package.name
                );
            }
        }
        return Ok(());
    }
    let data_dir = knowledge::data_dir();
    let packages = knowledge::list_packages(&data_dir);
    if json {
        println!("{}", serde_json::to_string_pretty(&packages)?);
        return Ok(());
    }
    println!("Knowledge Packages\n");
    println!("Data directory\n  {}\n", data_dir.display());
    if packages.is_empty() {
        println!("No knowledge packages installed.");
        return Ok(());
    }
    println!("Installed");
    for package in &packages {
        println!(
            "  {} {} ({} documents; integrity: {})",
            package.name, package.version, package.documents, package.integrity
        );
        if !package.description.is_empty() {
            println!("    {}", package.description);
        }
    }
    Ok(())
}

fn remove(name: &str, yes: bool) -> Result<()> {
    let data_dir = knowledge::data_dir();
    if !knowledge::packages::is_valid_package_name(name) {
        bail!("invalid package name");
    }
    if !yes {
        bail!(
            "removing package {name} deletes its entire directory, including any local changes; confirm with `lbc knowledge remove {name} --yes`; nothing was removed"
        );
    }
    let removed = knowledge::remove_package(name, &data_dir)
        .with_context(|| format!("failed to remove package {name:?}"))?;
    println!(
        "✓ Removed knowledge package {name}\n\nLocation\n  {}",
        removed.display()
    );
    Ok(())
}
