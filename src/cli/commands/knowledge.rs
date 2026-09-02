use std::path::Path;

use anyhow::{Context, Result};

use crate::cli::args::KnowledgeCommand;
use crate::knowledge;

pub fn run(command: KnowledgeCommand) -> Result<()> {
    match command {
        KnowledgeCommand::Install { source } => install(&source),
        KnowledgeCommand::List => list(),
        KnowledgeCommand::Remove { name } => remove(&name),
    }
}

fn install(source: &Path) -> Result<()> {
    let data_dir = knowledge::data_dir();
    let installed = knowledge::install_package(source, &data_dir)?;
    println!("Installed knowledge package\n");
    println!("  Name:        {}", installed.name);
    println!("  Version:     {}", installed.version);
    println!("  Documents:   {}", installed.documents);
    println!("  Location:    {}", installed.path.display());
    Ok(())
}

fn list() -> Result<()> {
    let data_dir = knowledge::data_dir();
    let packages = knowledge::list_packages(&data_dir);
    println!("Knowledge Packages\n");
    println!("Data directory\n  {}\n", data_dir.display());
    if packages.is_empty() {
        println!("No knowledge packages installed.");
        return Ok(());
    }
    println!("Installed");
    for package in &packages {
        println!(
            "  {} {} ({} documents)",
            package.name, package.version, package.documents
        );
        if !package.description.is_empty() {
            println!("    {}", package.description);
        }
    }
    Ok(())
}

fn remove(name: &str) -> Result<()> {
    let data_dir = knowledge::data_dir();
    let removed = knowledge::remove_package(name, &data_dir)
        .with_context(|| format!("failed to remove package {name:?}"))?;
    println!(
        "✓ Removed knowledge package {name}\n\nLocation\n  {}",
        removed.display()
    );
    Ok(())
}
