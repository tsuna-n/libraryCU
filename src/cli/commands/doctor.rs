use std::{fs, io::ErrorKind, path::Path};

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::cli::{args::DoctorArgs, errors::safe_message};
use crate::config::AiConfig;
use crate::{config, knowledge, scanner};

fn text<'a>(thai: bool, english: &'a str, translated: &'a str) -> &'a str {
    if thai { translated } else { english }
}

fn guidance(thai: bool, cause: &str, remedy: &str, next: &str) -> String {
    format!(
        "{cause}; {}: {remedy}; {}: {next}",
        text(thai, "Fix", "วิธีแก้"),
        text(thai, "Next", "คำสั่งถัดไป")
    )
}

fn load_config() -> Result<config::LoadedConfig> {
    let path = config::config_path();
    crate::security::files::reject_symlinks(&path)?;
    match fs::symlink_metadata(&path) {
        Ok(metadata) if !metadata.is_file() => bail!("configuration must be a regular file"),
        Err(error) if error.kind() != ErrorKind::NotFound => return Err(error.into()),
        _ => {}
    }
    config::load()
}

fn knowledge_check(root: &Path, thai: bool) -> DoctorCheck {
    let mut missing = Vec::new();
    let mut problems = Vec::new();
    for path in [
        knowledge::notes_dir(),
        knowledge::data_dir(),
        root.join("knowledge"),
        root.join(".lbc/knowledge"),
    ] {
        let inspected = (|| -> Result<bool> {
            crate::security::files::reject_symlinks(&path)?;
            match fs::symlink_metadata(&path) {
                Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
                Err(error) => return Err(error.into()),
                Ok(metadata) => anyhow::ensure!(
                    metadata.is_dir(),
                    "{}",
                    text(thai, "expected a directory", "ต้องเป็นไดเรกทอรี")
                ),
            }
            for entry in fs::read_dir(&path).context("cannot read directory")? {
                entry.context("cannot read directory entry")?;
            }
            Ok(true)
        })();
        match inspected {
            Ok(false) => missing.push(path.display().to_string()),
            Ok(true) => {}
            Err(error) => problems.push(format!("{}: {}", path.display(), safe_message(&error))),
        }
    }
    let loaded = knowledge::load_all_documents(root);
    let (available, invalid) = match &loaded {
        Ok(report) => (
            report
                .documents
                .iter()
                .filter(|document| document.effective)
                .count(),
            report.invalid.len(),
        ),
        Err(_) => (0, 0),
    };
    if let Ok(report) = &loaded {
        if let Some(first) = report.invalid.first() {
            problems.push(format!(
                "{invalid} {}; {}: {}",
                text(
                    thai,
                    "invalid documents or stores",
                    "เอกสารหรือแหล่งความรู้มีปัญหา"
                ),
                first.path,
                first.error
            ));
        }
    } else if let Err(error) = loaded {
        problems.push(safe_message(&error));
    }
    if available == 0 {
        problems.push(text(thai, "no knowledge documents available", "ไม่พบเอกสารความรู้").into());
    }
    let ok = problems.is_empty();
    let mut detail = format!(
        "{available} {}",
        text(
            thai,
            "valid/effective documents available",
            "เอกสารที่ใช้งานได้"
        )
    );
    if !missing.is_empty() {
        detail.push_str(&format!(
            "; {}: {}",
            text(
                thai,
                "optional stores not created (normal on first use)",
                "ยังไม่ได้สร้างแหล่งความรู้เสริม (ปกติเมื่อเริ่มใช้งาน)"
            ),
            missing.join(", ")
        ));
    }
    if !ok {
        detail = guidance(
            thai,
            &format!("{detail}; {}", problems.join("; ")),
            text(
                thai,
                "inspect the named path; preserve existing files, correct its directory type/read permissions or document metadata; symlinked stores are refused",
                "ตรวจสอบพาธที่ระบุ เก็บไฟล์เดิมไว้ แล้วแก้ชนิดไดเรกทอรี สิทธิ์อ่าน หรือข้อมูลเอกสาร ไม่รองรับแหล่งความรู้ที่เป็น symlink",
            ),
            "lbc index --json; lbc doctor",
        );
    }
    DoctorCheck {
        name: "Local knowledge".into(),
        ok,
        detail,
    }
}

fn ai_check(ai: &AiConfig, thai: bool) -> DoctorCheck {
    let result = crate::ai::credential(ai, |name| std::env::var(name).ok());
    DoctorCheck {
        name: "AI provider".into(),
        ok: result.is_ok(),
        detail: match result {
            Err(error) => guidance(
                thai,
                &safe_message(&error),
                text(
                    thai,
                    "set the provider API key in your environment, or use lbc config set ai.provider off for offline mode",
                    "ตั้งค่า API key ของผู้ให้บริการในสภาพแวดล้อม หรือใช้ lbc config set ai.provider off เพื่อทำงานออฟไลน์",
                ),
                "lbc config show; lbc doctor; lbc doctor --connectivity",
            ),
            Ok(_) if ai.provider == "off" => text(
                thai,
                "off (deterministic mode); no API key or network required",
                "off (โหมดกำหนดผลลัพธ์แน่นอน); ไม่ต้องใช้ API key หรือเครือข่าย",
            )
            .into(),
            Ok(key) => format!(
                "{}; model {}; URL {} ({}); {}",
                if matches!(ai.provider.as_str(), "zai" | "glm") {
                    "zai (glm)"
                } else {
                    &ai.provider
                },
                ai.effective_model(),
                if ai.provider == "openrouter" {
                    "https://openrouter.ai/api/v1"
                } else {
                    ai.effective_base_url()
                },
                if key.is_some() {
                    text(
                        thai,
                        "API key detected; validity untested",
                        "พบ API key; ยังไม่ได้ตรวจสอบความถูกต้อง",
                    )
                } else {
                    text(thai, "no API key required", "ไม่จำเป็นต้องใช้ API key")
                },
                text(
                    thai,
                    "connectivity untested; next: lbc doctor --connectivity",
                    "ยังไม่ได้ทดสอบการเชื่อมต่อ; คำสั่งถัดไป: lbc doctor --connectivity"
                )
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
    let loaded = load_config();
    let thai = crate::cli::errors::language() == "th";
    checks.push(match &loaded {
        Ok(loaded) => DoctorCheck {
            name: "Configuration".to_owned(),
            ok: true,
            detail: if loaded.found {
                format!(
                    "{} {}; {}: lbc config show",
                    text(thai, "loaded", "โหลดการตั้งค่าจาก"),
                    loaded.path.display(),
                    text(thai, "Next", "คำสั่งถัดไป")
                )
            } else {
                format!(
                    "{} {}; {}: lbc setup; lbc config show",
                    text(
                        thai,
                        "using safe defaults; optional configuration not created at",
                        "ใช้การตั้งค่าเริ่มต้นที่ปลอดภัย ยังไม่ได้สร้างไฟล์การตั้งค่าที่"
                    ),
                    loaded.path.display(),
                    text(thai, "Next", "คำสั่งถัดไป")
                )
            },
        },
        Err(error) => DoctorCheck {
            name: "Configuration".to_owned(),
            ok: false,
            detail: guidance(
                thai,
                &format!("{}: {}", config::config_path().display(), safe_message(error)),
                text(
                    thai,
                    "edit the configuration file to correct its settings, file type, read permissions, or symlink path; do not replace existing data",
                    "แก้ค่าการตั้งค่า ชนิดไฟล์ สิทธิ์อ่าน หรือพาธ symlink โดยเก็บข้อมูลเดิมไว้"
                ),
                "lbc doctor; lbc config show"
            ),
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
            detail: guidance(
                thai,
                &safe_message(error),
                text(
                    thai,
                    "run from an existing readable project directory",
                    "เรียกใช้จากไดเรกทอรีโครงการที่มีอยู่และอ่านได้",
                ),
                "lbc scan --path .; lbc doctor",
            ),
        },
    });

    checks.push(match &project {
        Ok(project) => knowledge_check(&project.root, thai),
        Err(_) => DoctorCheck {
            name: "Local knowledge".to_owned(),
            ok: false,
            detail: guidance(
                thai,
                text(thai, "project root is unavailable", "ไม่พบรากโครงการ"),
                text(
                    thai,
                    "run from an existing readable project directory",
                    "เรียกใช้จากไดเรกทอรีโครงการที่มีอยู่และอ่านได้",
                ),
                "lbc scan --path .; lbc doctor",
            ),
        },
    });

    checks.push(match &project {
        Ok(project) => match crate::security::git::check(&project.root) {
            Ok(report) => DoctorCheck {
                name: "Git safety".into(),
                ok: report.ok,
                detail: report.detail,
            },
            Err(error) => DoctorCheck {
                name: "Git safety".into(),
                ok: false,
                detail: guidance(
                    thai,
                    &safe_message(&error),
                    text(
                        thai,
                        "inspect Git rules and .lbc paths without replacing private records",
                        "ตรวจสอบกฎ Git และพาธ .lbc โดยเก็บข้อมูลส่วนตัวเดิมไว้",
                    ),
                    "git ls-files -- .lbc; lbc doctor",
                ),
            },
        },
        Err(_) => DoctorCheck {
            name: "Git safety".into(),
            ok: false,
            detail: text(
                thai,
                "project root unavailable; fix Project detection, then run lbc doctor",
                "ไม่พบรากโครงการ แก้การตรวจหาโครงการแล้วเรียก lbc doctor",
            )
            .into(),
        },
    });

    let ai_config = loaded
        .as_ref()
        .map(|loaded| loaded.config.ai.clone())
        .unwrap_or_default();
    checks.push(if loaded.is_err() {
        DoctorCheck {
            name: "AI provider".into(),
            ok: false,
            detail: text(
                thai,
                "configuration invalid; provider settings could not be checked; fix Configuration, then run lbc doctor",
                "การตั้งค่าไม่ถูกต้อง จึงตรวจสอบผู้ให้บริการไม่ได้ แก้การตั้งค่าแล้วเรียก lbc doctor"
            ).into(),
        }
    } else {
        ai_check(&ai_config, thai)
    });
    let connectivity = if loaded.is_err()
        || ai_config.provider == "off"
        || !checks.last().is_some_and(|c| c.ok)
    {
        Connectivity {
            status: "unavailable",
            detail: if loaded.is_err() {
                text(
                    thai,
                    "configuration invalid; no connection attempted; fix Configuration, then run lbc doctor --connectivity",
                    "การตั้งค่าไม่ถูกต้อง ยังไม่ได้เชื่อมต่อ แก้การตั้งค่าแล้วเรียก lbc doctor --connectivity"
                )
            } else if ai_config.provider == "off" {
                text(
                    thai,
                    "AI off; no connection attempted; offline mode is ready",
                    "AI ปิดอยู่ ยังไม่ได้เชื่อมต่อ พร้อมใช้งานออฟไลน์"
                )
            } else {
                text(
                    thai,
                    "credentials missing; no connection attempted; set the provider API key, then run lbc doctor --connectivity",
                    "ไม่พบข้อมูลรับรอง ยังไม่ได้เชื่อมต่อ ตั้งค่า API key แล้วเรียก lbc doctor --connectivity"
                )
            }.into(),
        }
    } else if !args.connectivity {
        Connectivity {
            status: "untested",
            detail: text(
                thai,
                "no connection attempted; opt in with lbc doctor --connectivity (models endpoint only)",
                "ยังไม่ได้เชื่อมต่อ หากต้องการตรวจสอบให้เรียก lbc doctor --connectivity (เฉพาะรายการโมเดล)"
            ).into(),
        }
    } else {
        match crate::ai::connectivity::check(&ai_config, args.connectivity_timeout) {
            Ok(()) => Connectivity {
                status: "available",
                detail: text(
                    thai,
                    "models endpoint responded; generation and configured model availability untested",
                    "ปลายทางรายการโมเดลตอบกลับแล้ว ยังไม่ได้ทดสอบการสร้างคำตอบหรือโมเดลที่ตั้งค่าไว้"
                ).into(),
            },
            Err(error) => Connectivity {
                status: "error",
                detail: guidance(
                    thai,
                    &safe_message(&error),
                    text(
                        thai,
                        "check provider URL, API key, service availability, and network; a successful probe does not verify generation",
                        "ตรวจสอบ URL, API key, สถานะบริการ และเครือข่าย การเชื่อมต่อสำเร็จยังไม่ยืนยันการสร้างคำตอบ"
                    ),
                    "lbc config show; lbc doctor --connectivity"
                ),
            },
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
                "{} {}",
                text(thai, "persistent mode configured; history not opened or changed by doctor; bounded redacted history at", "ตั้งค่าโหมดบันทึกถาวร doctor ไม่เปิดหรือเปลี่ยนแปลงประวัติที่"),
                crate::history::history_path().display()
            )
        } else {
            format!(
                "{} {}",
                text(thai, "session-only mode; no history is written to", "โหมดเฉพาะเซสชัน ไม่มีการบันทึกประวัติที่"),
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
                if thai {
                    match check.name.as_str() {
                        "Configuration" => "การตั้งค่า",
                        "Project detection" => "การตรวจหาโครงการ",
                        "Local knowledge" => "ความรู้ในเครื่อง",
                        "AI provider" => "ผู้ให้บริการ AI",
                        "Session memory" => "หน่วยความจำเซสชัน",
                        "Git safety" => "ความปลอดภัยของ Git",
                        name => name,
                    }
                } else {
                    &check.name
                },
                check.detail
            );
        }
        println!(
            "{}: {}\n  {}\n",
            text(thai, "Provider connectivity", "การเชื่อมต่อผู้ให้บริการ"),
            report.provider_connectivity.status,
            report.provider_connectivity.detail
        );
        println!(
            "{}: {}",
            text(thai, "Health scope", "ขอบเขตการตรวจสอบ"),
            if thai {
                if args.connectivity {
                    "ตรวจสอบในเครื่องและปลายทางรายการโมเดล"
                } else {
                    "ตรวจสอบในเครื่องเท่านั้น ยังไม่ได้ทดสอบการเชื่อมต่อผู้ให้บริการ"
                }
            } else {
                report.health_scope
            }
        );
        println!(
            "{}\n  {}",
            text(thai, "Status", "สถานะ"),
            if report.healthy {
                text(thai, "healthy", "พร้อมใช้งาน")
            } else {
                text(thai, "attention required", "ต้องแก้ไขรายการที่มีปัญหา")
            }
        );
    }
    if !report.healthy {
        bail!(
            "{}",
            text(
                thai,
                "doctor found problems; review the checks above, then run lbc doctor",
                "doctor พบปัญหา ตรวจสอบวิธีแก้ข้างต้น แล้วเรียก lbc doctor อีกครั้ง"
            )
        );
    }
    Ok(())
}
