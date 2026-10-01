use anyhow::{Result, bail};
use serde::Serialize;

use crate::{
    answer,
    cli::args::{ApplyProposalArgs, ExplainArgs, FixArgs, RollbackArgs},
    config, diagnostics, fixer, security,
};

#[derive(Serialize)]
struct Application {
    status: &'static str,
    applied: bool,
    verification_status: &'static str,
    recovery_id: Option<String>,
    verification: Option<fixer::verification::Verification>,
    rollback_status: Option<&'static str>,
}

impl Default for Application {
    fn default() -> Self {
        Self {
            status: "offline_guidance",
            applied: false,
            verification_status: "unverified",
            recovery_id: None,
            verification: None,
            rollback_status: None,
        }
    }
}

#[derive(Serialize)]
struct FixReport {
    #[serde(flatten)]
    application: Application,
    guidance: answer::AnswerReport,
    patch: Option<fixer::Patch>,
    error: Option<String>,
    proposal_id: Option<String>,
}

pub fn run(args: FixArgs) -> Result<()> {
    let verify = args
        .verify
        .as_deref()
        .map(fixer::verification::parse)
        .transpose()?;
    let input = super::explain::read_error_input(&ExplainArgs {
        file: args.file,
        stdin: args.stdin,
        ai: args.ai,
        json: args.json,
        verbose: false,
        project: args.project.clone(),
    })?;
    let diagnostic = diagnostics::parse_primary(&input)
        .ok_or_else(|| anyhow::anyhow!("error input is empty"))?;
    let loaded = config::load()?;
    security::files::reject_symlinks(&args.project)?;
    let root = args.project.canonicalize()?;
    anyhow::ensure!(root.is_dir(), "--project must name a directory");
    let query = format!(
        "{} {}",
        diagnostic.code.as_deref().unwrap_or_default(),
        diagnostic.message
    );
    let mut output = loaded.config.output.clone();
    output.language = answer::choose_language(&output.language, &input);
    let guidance = answer::answer(query.trim(), &root, &output, &loaded.config.scanner)?;
    let mut report = FixReport {
        application: Application::default(),
        guidance,
        patch: None,
        error: None,
        proposal_id: None,
    };
    if args.ai {
        let generated = (|| -> Result<()> {
            let target = fixer::Target::read(&root, &diagnostic)?;
            let mut patch = target.generate(&report.guidance, &loaded.config.ai)?;
            if args.save_proposal {
                report.proposal_id = Some(fixer::proposal::save(&target, &patch)?);
            }
            report.application.status = "proposed";
            let applied = if args.apply {
                apply_and_verify(
                    &root,
                    &target,
                    &mut patch,
                    verify.as_ref(),
                    args.verify_timeout,
                    &mut report.application,
                )
            } else {
                Ok(())
            };
            report.patch = Some(patch);
            applied?;
            Ok(())
        })();
        if let Err(error) = generated {
            if report.application.status != "rolled_back" {
                report.application.status = "failed";
            }
            report.error = Some(security::redact_sensitive(&format!("{error:#}")));
        }
    }
    for warning in &report.guidance.warnings {
        eprintln!("! {warning}");
    }
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        let thai = report.guidance.language == "th";
        if let Some(patch) = &report.patch {
            println!(
                "{}: {}\n{}:\n{}\n{}:\n{}",
                if thai { "แก้" } else { "Change" },
                patch.path,
                if thai { "จาก" } else { "From" },
                patch.before,
                if thai { "เป็น" } else { "To" },
                patch.after
            );
            if let Some(verification) = &report.application.verification {
                println!(
                    "{}: {}",
                    if thai {
                        "ผลการตรวจสอบ"
                    } else {
                        "Verification"
                    },
                    verification.status
                );
                if let Some(error) = &verification.error {
                    eprintln!("! {error}");
                }
                if !verification.stdout.is_empty() {
                    println!("{}", verification.stdout);
                }
                if !verification.stderr.is_empty() {
                    eprintln!("{}", verification.stderr);
                }
            }
            if let Some(id) = &report.proposal_id {
                println!("Proposal ID: {id}");
            }
            if let Some(id) = &report.application.recovery_id {
                println!("Recovery ID: {id}");
            }
            if let Some(status) = report.application.rollback_status {
                println!("Rollback: {status}");
            }
            if report.application.verification.is_none() {
                println!(
                    "{}",
                    match (
                        report.application.applied,
                        thai,
                        report.proposal_id.is_some()
                    ) {
                        (false, true, true) =>
                            "บันทึกข้อเสนอแล้ว; ยังไม่ได้แก้ source ของ project หรือรันการตรวจสอบ",
                        (false, false, true) =>
                            "Proposal saved; no project source changed or verification commands run.",
                        (true, true, _) => "เขียนไฟล์แล้ว; ยังไม่ได้รันการตรวจสอบ",
                        (false, true, false) => "ข้อเสนอเท่านั้น; ยังไม่ได้เขียนไฟล์หรือรันการตรวจสอบ",
                        (true, false, _) => "Applied; no verification commands were run.",
                        (false, false, false) =>
                            "Proposal only; no files changed or verification commands run.",
                    }
                );
            }
        } else {
            println!("{}", report.guidance.offline_answer);
            if !args.ai {
                println!(
                    "{}",
                    if thai {
                        "ใช้ --ai เพื่อขอข้อเสนอแก้ไข และเพิ่ม --apply เมื่อต้องการเขียนไฟล์"
                    } else {
                        "Use --ai to request a patch; add --apply to write the validated replacement."
                    }
                );
            }
        }
    }
    if let Some(error) = report.error {
        bail!("{error}");
    }
    Ok(())
}

fn apply_and_verify(
    root: &std::path::Path,
    target: &fixer::Target,
    patch: &mut fixer::Patch,
    verify: Option<&Vec<String>>,
    timeout: u64,
    report: &mut Application,
) -> Result<()> {
    report.recovery_id = Some(fixer::rollback::prepare(target, patch)?);
    target.apply(patch)?;
    report.applied = true;
    report.status = "applied";
    if let Some(command) = verify {
        let mut verification = fixer::verification::run(root, command, timeout);
        if verification.status == "passed"
            && let Err(error) = target.check_applied(patch)
        {
            verification.status = "error";
            verification.error = Some(security::redact_sensitive(&format!("{error:#}")));
        }
        report.verification_status = verification.status;
        report.verification = Some(verification);
        if report.verification_status == "passed" {
            report.status = "verified";
            patch.verification_status = "passed";
        } else {
            if !report
                .verification
                .as_ref()
                .is_some_and(|v| v.process_stopped)
            {
                report.rollback_status = Some("failed");
                bail!(
                    "verification process termination could not be confirmed; stop it before using the recovery record"
                );
            }
            match fixer::rollback::restore(
                root,
                report.recovery_id.as_deref().expect("recorded application"),
            ) {
                Ok(_) => {
                    report.applied = false;
                    report.status = "rolled_back";
                    report.rollback_status = Some("succeeded");
                }
                Err(error) => {
                    report.rollback_status = Some("failed");
                    bail!("verification did not pass; rollback failed: {error:#}");
                }
            }
            bail!("verification did not pass; original source restored");
        }
    }
    Ok(())
}

pub fn apply_proposal(args: ApplyProposalArgs) -> Result<()> {
    let verify = args
        .verify
        .as_deref()
        .map(fixer::verification::parse)
        .transpose()?;
    let mut application = Application::default();
    let mut patch = None;
    let result = (|| -> Result<()> {
        let (target, mut saved) = fixer::proposal::load(&args.project, &args.id)?;
        let root = args.project.canonicalize()?;
        let result = apply_and_verify(
            &root,
            &target,
            &mut saved,
            verify.as_ref(),
            args.verify_timeout,
            &mut application,
        );
        patch = Some(saved);
        result
    })();
    let error = result
        .as_ref()
        .err()
        .map(|e| security::redact_sensitive(&format!("{e:#}")));
    if error.is_some() && application.status != "rolled_back" {
        application.status = "failed";
    }
    if args.json {
        #[derive(Serialize)]
        struct SavedReport<'a> {
            #[serde(flatten)]
            application: &'a Application,
            proposal_id: String,
            patch: &'a Option<fixer::Patch>,
            error: &'a Option<String>,
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&SavedReport {
                application: &application,
                proposal_id: security::redact_sensitive(&args.id),
                patch: &patch,
                error: &error,
            })?
        );
    } else {
        println!(
            "Proposal: {}\nStatus: {}\nVerification: {}",
            security::redact_sensitive(&args.id),
            application.status,
            application.verification_status
        );
        if let Some(patch) = &patch {
            println!(
                "Change: {}\nFrom:\n{}\nTo:\n{}",
                patch.path, patch.before, patch.after
            );
        }
        if let Some(id) = &application.recovery_id {
            println!("Recovery ID: {id}");
        }
        if let Some(v) = &application.verification {
            if !v.stdout.is_empty() {
                println!("{}", v.stdout);
            }
            if !v.stderr.is_empty() {
                eprintln!("{}", v.stderr);
            }
        }
        if let Some(status) = application.rollback_status {
            println!("Rollback: {status}");
        }
    }
    if let Some(error) = error {
        bail!("{error}");
    }
    Ok(())
}

pub fn rollback(args: RollbackArgs) -> Result<()> {
    let result = fixer::rollback::restore(&args.project, &args.id);
    let error = result
        .as_ref()
        .err()
        .map(|e| security::redact_sensitive(&format!("{e:#}")));
    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "status": if result.is_ok() { "rolled_back" } else { "failed" },
                "recovery_id": security::redact_sensitive(&args.id),
                "path": result.as_ref().ok().map(|p| security::redact_sensitive(p)),
                "error": error,
            })
        );
    } else if let Ok(path) = &result {
        println!("Restored: {}", security::redact_sensitive(path));
    }
    result.map(|_| ())
}
