use std::{
    fs,
    io::{self, BufRead, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;

use crate::{cli::args::SetupArgs, config, knowledge, scanner, security};

const MAX_FIELD_BYTES: usize = 16 * 1024;

pub fn run(args: SetupArgs) -> Result<()> {
    let project = scanner::find_project_root(args.project.as_deref().unwrap_or(Path::new(".")))?;
    let detected = scanner::detect_project(&project)?;
    let project_knowledge = project.join(".lbc/knowledge");
    let personal_knowledge = knowledge::notes_dir();
    let path = config::config_path();
    // Preflight every destination before prompting or creating any store.
    check_paths(&path, [&project_knowledge, &personal_knowledge])?;
    config::load()?;
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut prompts = io::stderr().lock();
    if !args.yes {
        writeln!(
            prompts,
            "libraryCube setup\nOS: {}\nProject: {}\nDetected stack: {}\nConfig: {}\nProject knowledge: {}\nPersonal knowledge: {}",
            std::env::consts::OS,
            project.display(),
            detected.stack_label(),
            path.display(),
            project_knowledge.display(),
            personal_knowledge.display(),
        )?;
        match prompt(
            &mut input,
            &mut prompts,
            "Save configuration and initialize knowledge stores? [Y/n]",
        )?
        .to_lowercase()
        .as_str()
        {
            "" | "y" | "yes" => {}
            "n" | "no" => {
                writeln!(prompts, "Cancelled; nothing was saved.")?;
                return Ok(());
            }
            _ => bail!("expected yes or no; nothing was saved"),
        }
    }
    let loaded = config::settings::update(
        |_| Ok(()),
        || {
            check_paths(&path, [&project_knowledge, &personal_knowledge])?;
            for directory in [&project_knowledge, &personal_knowledge] {
                fs::create_dir_all(directory)
                    .with_context(|| format!("failed to create {}", directory.display()))?;
                security::permissions::validate_directory(directory)?;
            }
            security::git::ensure_gitignore(&project)?;
            Ok(())
        },
    )?;
    let report = SetupReport {
        os: std::env::consts::OS,
        project,
        stack: detected.stack_label(),
        config_path: loaded.path,
        project_knowledge,
        personal_knowledge,
    };
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "Setup complete\nOS: {}\nProject: {}\nDetected stack: {}\nConfig: {}\nProject knowledge: {}\nPersonal knowledge: {}",
            report.os,
            report.project.display(),
            report.stack,
            report.config_path.display(),
            report.project_knowledge.display(),
            report.personal_knowledge.display(),
        );
        println!("\nSave: lbc learn\nFind: lbc search <query>\nReuse: lbc explain <log>");
    }
    Ok(())
}

#[derive(Serialize)]
struct SetupReport {
    os: &'static str,
    project: PathBuf,
    stack: String,
    config_path: PathBuf,
    project_knowledge: PathBuf,
    personal_knowledge: PathBuf,
}

fn check_paths(config: &Path, directories: [&PathBuf; 2]) -> Result<()> {
    security::files::reject_symlinks(config)?;
    security::files::reject_symlinks(&config.with_extension("lock"))?;
    security::permissions::validate_directory(
        config.parent().context("config path has no parent")?,
    )?;
    for directory in directories {
        security::files::reject_symlinks(directory)?;
        ensure!(
            !directory.exists() || directory.is_dir(),
            "knowledge destination must be a directory"
        );
        security::permissions::validate_directory(directory)?;
    }
    let ignore = directories[0]
        .parent()
        .context("project knowledge has no parent")?
        .join(".gitignore");
    security::files::reject_symlinks(&ignore)?;
    if ignore.exists() {
        ensure!(ignore.is_file(), ".lbc/.gitignore must be a regular file");
        security::permissions::validate_path(&ignore, false)?;
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
    ensure!(
        bytes > 0,
        "setup input ended; nothing was saved (use --yes for scripts)"
    );
    ensure!(
        bytes <= MAX_FIELD_BYTES,
        "setup field exceeds 16 KB; nothing was saved"
    );
    Ok(line.trim().to_owned())
}
