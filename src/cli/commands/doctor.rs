use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::cli::args::DoctorArgs;
use crate::{config, knowledge, scanner};

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
        Ok(project) => match knowledge::load_documents(&project.root) {
            Ok(documents) if !documents.is_empty() => DoctorCheck {
                name: "Local knowledge".to_owned(),
                ok: true,
                detail: format!("{} documents available", documents.len()),
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

    let report = DoctorReport {
        healthy: checks.iter().all(|check| check.ok),
        checks,
    };
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("LCU Doctor\n");
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
    Ok(())
}
