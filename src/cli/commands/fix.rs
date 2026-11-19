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
    let input = super::explain::read_error_input(&ExplainArgs {
        file: args.file,
        text: None,
        stdin: args.stdin,
        json: args.json,
        verbose: false,
        project: args.project.clone(),
        project_only: false,
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
    let report = FixReport {
        application: Application::default(),
        guidance,
        patch: None,
        error: None,
        proposal_id: None,
    };
    for warning in &report.guidance.warnings {
        crate::output::warning(warning);
    }
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", report.guidance.offline_answer);
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
