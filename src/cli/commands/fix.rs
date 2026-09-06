use anyhow::{Result, bail};
use serde::Serialize;

use crate::{
    answer,
    cli::args::{ExplainArgs, FixArgs},
    config, diagnostics, fixer, security,
};

#[derive(Serialize)]
struct FixReport {
    status: &'static str,
    applied: bool,
    verification_status: &'static str,
    guidance: answer::AnswerReport,
    patch: Option<fixer::Patch>,
    error: Option<String>,
}

pub fn run(args: FixArgs) -> Result<()> {
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
        status: "offline_guidance",
        applied: false,
        verification_status: "unverified",
        guidance,
        patch: None,
        error: None,
    };
    if args.ai {
        let generated = (|| -> Result<()> {
            let target = fixer::Target::read(&root, &diagnostic)?;
            let patch = target.generate(&report.guidance, &loaded.config.ai)?;
            if args.apply {
                target.apply(&patch)?;
            }
            report.applied = args.apply;
            report.status = if args.apply { "applied" } else { "proposed" };
            report.patch = Some(patch);
            Ok(())
        })();
        if let Err(error) = generated {
            report.status = "failed";
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
            println!(
                "{}",
                match (report.applied, thai) {
                    (true, true) => "เขียนไฟล์แล้ว; ยังไม่ได้รันการตรวจสอบ",
                    (false, true) => "ข้อเสนอเท่านั้น; ยังไม่ได้เขียนไฟล์หรือรันการตรวจสอบ",
                    (true, false) => "Applied; no verification commands were run.",
                    (false, false) =>
                        "Proposal only; no files changed or verification commands run.",
                }
            );
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
