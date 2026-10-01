use std::path::Path;

use anyhow::{Result, bail};
use serde::Serialize;

use crate::cli::args::DoctorArgs;
use crate::config::AiConfig;
use crate::{config, knowledge, scanner};

fn ai_check(ai: &AiConfig) -> DoctorCheck {
    let result = crate::ai::credential(ai, |name| std::env::var(name).ok());
    DoctorCheck {
        name: "AI provider".into(),
        ok: result.is_ok(),
        detail: match result {
            Err(error) => format!("{error}"),
            Ok(_) if ai.provider == "off" => "off (deterministic mode)".into(),
            Ok(key) => format!(
                "{} with model {} at {} ({}); connectivity untested",
                if matches!(ai.provider.as_str(), "zai" | "glm") {
                    "zai (glm)"
                } else {
                    &ai.provider
                },
                ai.effective_model(),
                ai.effective_base_url(),
                if key.is_some() {
                    "API key detected; validity untested"
                } else {
                    "no API key required"
                }
            ),
        },
    }
}

#[derive(Debug, Serialize)]
struct Connectivity {
    status: &'static str,
    detail: String,
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
    health_scope: &'static str,
    provider_connectivity: Connectivity,
}

pub fn run(args: DoctorArgs) -> Result<()> {
    let mut checks = Vec::new();
    let loaded = config::load();
    checks.push(match &loaded {
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

    let ai_config = loaded
        .as_ref()
        .map(|loaded| loaded.config.ai.clone())
        .unwrap_or_default();
    checks.push(ai_check(&ai_config));
    let connectivity = if loaded.is_err()
        || ai_config.provider == "off"
        || !checks.last().is_some_and(|c| c.ok)
    {
        Connectivity {
            status: "unavailable",
            detail:
                "configuration invalid, credentials missing, or AI off; no connection attempted"
                    .into(),
        }
    } else if !args.connectivity {
        Connectivity {
            status: "untested",
            detail: "no connection attempted; opt in with --connectivity".into(),
        }
    } else {
        match crate::ai::connectivity::check(&ai_config, args.connectivity_timeout) {
            Ok(()) => Connectivity { status: "available", detail: "models endpoint responded; generation and configured model availability untested".into() },
            Err(error) => Connectivity { status: "error", detail: crate::security::redact_sensitive(&format!("{error:#}")) },
        }
    };
    let memory = loaded
        .as_ref()
        .map(|loaded| loaded.config.memory.clone())
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
        healthy: checks.iter().all(|check| check.ok)
            && (!args.connectivity || connectivity.status == "available"),
        health_scope: if args.connectivity {
            "local checks and provider models endpoint"
        } else {
            "local checks only; provider connectivity untested"
        },
        provider_connectivity: connectivity,
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
            "Provider connectivity: {}\n  {}\n",
            report.provider_connectivity.status, report.provider_connectivity.detail
        );
        println!("Health scope: {}", report.health_scope);
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
