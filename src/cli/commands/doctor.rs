use std::path::Path;

use anyhow::{Result, bail};
use serde::Serialize;

use crate::cli::args::DoctorArgs;
use crate::config::AiConfig;
use crate::{config, knowledge, scanner};

fn ai_check(ai: &AiConfig) -> DoctorCheck {
    match ai.provider.as_str() {
        "off" => DoctorCheck {
            name: "AI provider".to_owned(),
            ok: true,
            detail: "off (deterministic mode); set ai.provider to enable AI explanations"
                .to_owned(),
        },
        "openrouter" => {
            let key_set =
                std::env::var("OPENROUTER_API_KEY").is_ok_and(|key| !key.trim().is_empty());
            DoctorCheck {
                name: "AI provider".to_owned(),
                ok: key_set,
                detail: if key_set {
                    format!("openrouter with model {} (API key detected)", ai.model)
                } else {
                    format!(
                        "openrouter with model {} but OPENROUTER_API_KEY is not set",
                        ai.model
                    )
                },
            }
        }
        "openai-compat" => {
            let key_status = if ["GLM_API_KEY", "ZAI_API_KEY", "OPENAI_API_KEY"]
                .iter()
                .any(|key| std::env::var_os(key).is_some())
            {
                "API key detected"
            } else {
                "no API key (fine for local servers such as Ollama)"
            };
            DoctorCheck {
                name: "AI provider".to_owned(),
                ok: true,
                detail: format!(
                    "openai-compat with model {} at {} ({key_status})",
                    ai.model, ai.base_url
                ),
            }
        }
        other => DoctorCheck {
            name: "AI provider".to_owned(),
            ok: false,
            detail: format!("unsupported provider {other:?}"),
        },
    }
}

#[derive(Debug, Serialize)]
struct DoctorCheck {
    name: String,
    ok: bool,
    detail: String,
}

#[derive(Debug, Serialize)]
struct DoctorReport {
    healthy: bool,
    checks: Vec<DoctorCheck>,
}

pub fn run(args: DoctorArgs) -> Result<()> {
    let mut checks = Vec::new();
    checks.push(match config::load() {
        Ok(loaded) => DoctorCheck {
            name: "Configuration".to_owned(),
            ok: true,
            detail: if loaded.found {
                format!("loaded {}", loaded.path.display())
            } else {
                "using safe defaults".to_owned()
            },
        },
        Err(error) => DoctorCheck {
            name: "Configuration".to_owned(),
            ok: false,
            detail: error.to_string(),
        },
    });

    let project = scanner::detect_project(Path::new("."));
    checks.push(match &project {
        Ok(project) => DoctorCheck {
            name: "Project detection".to_owned(),
            ok: true,
            detail: format!("{} at {}", project.stack_label(), project.root.display()),
        },
        Err(error) => DoctorCheck {
            name: "Project detection".to_owned(),
            ok: false,
            detail: error.to_string(),
        },
    });

    checks.push(match project {
        Ok(project) => match knowledge::load_all_documents(&project.root) {
            Ok(report) if !report.invalid.is_empty() => DoctorCheck {
                name: "Local knowledge".to_owned(),
                ok: false,
                detail: format!(
                    "{} valid/effective documents; {} invalid (first: {})",
                    report
                        .documents
                        .iter()
                        .filter(|document| document.effective)
                        .count(),
                    report.invalid.len(),
                    report.invalid[0].path
                ),
            },
            Ok(report) if !report.documents.is_empty() => DoctorCheck {
                name: "Local knowledge".to_owned(),
                ok: true,
                detail: format!("{} documents available", report.documents.len()),
            },
            Ok(_) => DoctorCheck {
                name: "Local knowledge".to_owned(),
                ok: false,
                detail: "no knowledge documents available".to_owned(),
            },
            Err(error) => DoctorCheck {
                name: "Local knowledge".to_owned(),
                ok: false,
                detail: error.to_string(),
            },
        },
        Err(_) => DoctorCheck {
            name: "Local knowledge".to_owned(),
            ok: false,
            detail: "project root is unavailable".to_owned(),
        },
    });

    let ai_config = config::load()
        .map(|loaded| loaded.config.ai)
        .unwrap_or_default();
    checks.push(ai_check(&ai_config));

    let memory = config::load()
        .map(|loaded| loaded.config.memory)
        .unwrap_or_default();
    checks.push(DoctorCheck {
        name: "Session memory".to_owned(),
        ok: true,
        detail: if memory.mode == "persistent" {
            format!(
                "persistent mode enabled; bounded redacted history at {}",
                crate::history::history_path().display()
            )
        } else {
            format!(
                "session-only mode; no history is written to {}",
                crate::history::history_path().display()
            )
        },
    });

    for check in &mut checks {
        check.detail = crate::security::redact_sensitive(&check.detail);
    }
    let report = DoctorReport {
        healthy: checks.iter().all(|check| check.ok),
        checks,
    };
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("libraryCube doctor\n");
        for check in &report.checks {
            println!(
                "{} {}\n  {}\n",
                if check.ok { "✓" } else { "!" },
                check.name,
                check.detail
            );
        }
        println!(
            "Status\n  {}",
            if report.healthy {
                "healthy"
            } else {
                "attention required"
            }
        );
    }
    if !report.healthy {
        bail!("doctor found problems; review the checks above");
    }
    Ok(())
}
