use std::{
    fs,
    io::{self, BufRead, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;

use crate::{cli::args::SetupArgs, config, knowledge, scanner, security};

const MAX_FIELD_BYTES: usize = 16 * 1024;

pub fn run(mut args: SetupArgs) -> Result<()> {
    let project = scanner::find_project_root(args.project.as_deref().unwrap_or(Path::new(".")))?;
    let detected = scanner::detect_project(&project)?;
    let project_knowledge = project.join(".lbc/knowledge");
    let personal_knowledge = knowledge::notes_dir();
    let path = config::config_path();
    // Preflight every destination before prompting or creating any store.
    check_paths(&path, [&project_knowledge, &personal_knowledge])?;
    let loaded = config::load()?;
    let mut candidate = loaded.config.clone();
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut prompts = io::stderr().lock();
    if !args.yes {
        writeln!(
            prompts,
            "libraryCube setup\nOS: {}\nProject: {}\nDetected stack: {}\nConfig: {}\nProject knowledge: {}\nPersonal knowledge: {}\nAI is optional. API keys belong only in environment variables; never enter a key here.",
            std::env::consts::OS,
            project.display(),
            detected.stack_label(),
            path.display(),
            project_knowledge.display(),
            personal_knowledge.display(),
        )?;
        if args.provider.is_none() {
            let value = prompt(
                &mut input,
                &mut prompts,
                &format!(
                    "AI provider: off / openai / zai / glm / ollama / openrouter / openai-compat [{}]",
                    candidate.ai.provider
                ),
            )?;
            if !value.is_empty() {
                args.provider = Some(value);
            }
        }
        if let Some(provider) = &args.provider {
            candidate.ai.provider = provider.clone();
        }
        ensure!(
            matches!(
                candidate.ai.provider.as_str(),
                "off" | "openai" | "zai" | "glm" | "ollama" | "openrouter" | "openai-compat"
            ),
            "unknown AI provider; nothing was saved"
        );
        if candidate.ai.provider != "off" {
            if args.model.is_none() {
                let value = prompt(
                    &mut input,
                    &mut prompts,
                    "Model (blank keeps current/default)",
                )?;
                if !value.is_empty() {
                    args.model = Some(value);
                }
            }
            if args.base_url.is_none() {
                let value = prompt(
                    &mut input,
                    &mut prompts,
                    "Base URL (blank keeps current/default)",
                )?;
                if !value.is_empty() {
                    args.base_url = Some(value);
                }
            }
        }
    }
    apply(&args, &mut candidate)?;
    config::settings::validate(&candidate)?;
    if !args.yes {
        writeln!(
            prompts,
            "AI provider: {}. No network checks will run.",
            candidate.ai.provider
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
        |config| apply(&args, config),
        || {
            check_paths(&path, [&project_knowledge, &personal_knowledge])?;
            for directory in [&project_knowledge, &personal_knowledge] {
                fs::create_dir_all(directory)
                    .with_context(|| format!("failed to create {}", directory.display()))?;
                security::permissions::validate_directory(directory)?;
            }
            Ok(())
        },
    )?;
    let ai = loaded.config.ai;
    let (environment_variables, required): (&[&str], bool) = match ai.provider.as_str() {
        "openai" => (&["OPENAI_API_KEY"], true),
        "zai" | "glm" => (&["ZAI_API_KEY", "GLM_API_KEY"], true),
        "openrouter" => (&["OPENROUTER_API_KEY"], true),
        "ollama" => (&["OLLAMA_API_KEY"], false),
        "openai-compat" => (&["GLM_API_KEY", "ZAI_API_KEY", "OPENAI_API_KEY"], false),
        _ => (&[], false),
    };
    let credentials = if ai.provider == "off" {
        "not needed; AI is off"
    } else {
        match crate::ai::credential(&ai, |name| std::env::var(name).ok()) {
            Ok(Some(_)) => "detected; validity untested",
            _ if required => "missing; offline knowledge remains available",
            _ => "optional; no key detected",
        }
    };
    let report = SetupReport {
        os: std::env::consts::OS,
        project,
        stack: detected.stack_label(),
        config_path: loaded.path,
        project_knowledge,
        personal_knowledge,
        provider: ai.provider,
        environment_variables,
        credentials,
        connectivity: "untested; no network requests made",
    };
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "Setup complete\nOS: {}\nProject: {}\nDetected stack: {}\nConfig: {}\nProject knowledge: {}\nPersonal knowledge: {}\nAI provider: {}\nCredentials: {}\nConnectivity: {}",
            report.os,
            report.project.display(),
            report.stack,
            report.config_path.display(),
            report.project_knowledge.display(),
            report.personal_knowledge.display(),
            report.provider,
            report.credentials,
            report.connectivity,
        );
        if !environment_variables.is_empty() {
            println!(
                "Environment variables: {} ({})",
                environment_variables.join(" or "),
                if required {
                    "required for AI"
                } else {
                    "optional for AI"
                }
            );
        }
        println!("\nSave: lbc learn\nFind: lbc search <query>\nReuse: lbc explain <log>");
    }
    Ok(())
}

#[derive(Serialize)]
struct SetupReport<'a> {
    os: &'static str,
    project: PathBuf,
    stack: String,
    config_path: PathBuf,
    project_knowledge: PathBuf,
    personal_knowledge: PathBuf,
    provider: String,
    environment_variables: &'a [&'a str],
    credentials: &'static str,
    connectivity: &'static str,
}

fn apply(args: &SetupArgs, config: &mut config::Config) -> Result<()> {
    if let Some(provider) = &args.provider {
        config.ai.provider = provider.clone();
    }
    if let Some(model) = &args.model {
        ensure!(
            !model.is_empty()
                && model.len() <= MAX_FIELD_BYTES
                && !model.chars().any(char::is_whitespace)
                && !model.contains(['@', '?', '#'])
                && !model.contains("://")
                && security::redact_sensitive(model) == *model,
            "model must be a model identifier without credentials; nothing was saved"
        );
        config.ai.model = model.clone();
    }
    if let Some(base_url) = &args.base_url {
        let url = reqwest::Url::parse(base_url).map_err(|_| {
            anyhow::anyhow!(
                "base URL must be an HTTP(S) URL without credentials; nothing was saved"
            )
        })?;
        ensure!(
            base_url.len() <= MAX_FIELD_BYTES
                && matches!(url.scheme(), "http" | "https")
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none()
                && !base_url.chars().any(char::is_whitespace)
                && security::redact_sensitive(base_url) == *base_url,
            "base URL must be an HTTP(S) URL without credentials, query, or fragment; nothing was saved"
        );
        config.ai.base_url = base_url.clone();
    }
    Ok(())
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
