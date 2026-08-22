use crate::diagnostics::ExplanationReport;

pub fn print_explanation(report: &ExplanationReport, verbose: bool) {
    let code = report.diagnostic.code.as_deref().unwrap_or("Unknown error");
    println!("LCU Diagnostic\n");
    println!("✗ {code} - {}\n", report.diagnostic.message);
    println!("Project\n  {}\n", report.project.stack_label());
    if let Some(file) = &report.diagnostic.file {
        let location = match (report.diagnostic.line, report.diagnostic.column) {
            (Some(line), Some(column)) => format!("{}:{line}:{column}", file.display()),
            (Some(line), None) => format!("{}:{line}", file.display()),
            _ => file.display().to_string(),
        };
        println!("Location\n  {location}\n");
    }
    println!("Evidence");
    for evidence in &report.evidence {
        println!("  - {evidence}");
    }
    println!("\nCause\n  {}", report.cause);

    if !report.suggested_fixes.is_empty() {
        println!("\nSuggested fix");
        for (index, fix) in report.suggested_fixes.iter().enumerate() {
            println!("  {}. {fix}", index + 1);
        }
    }
    if !report.next_steps.is_empty() {
        println!("\nPossible next steps");
        for (index, step) in report.next_steps.iter().enumerate() {
            println!("  {}. {step}", index + 1);
        }
    }
    if !report.verification.is_empty() {
        println!("\nVerify");
        for command in &report.verification {
            println!("  {command}");
        }
    }
    if !report.knowledge.is_empty() {
        println!("\nKnowledge");
        for item in &report.knowledge {
            if verbose {
                println!("  {} ({}, {})", item.title, item.path, item.match_reason);
            } else {
                println!("  {}", item.title);
            }
        }
    } else {
        println!("\nLocal knowledge\n  No exact match found.");
    }
    if verbose {
        println!("\nProject context");
        println!("  Files inspected: {}", report.files_inspected);
        if !report.project.frameworks.is_empty() {
            println!("  Frameworks: {}", report.project.frameworks.join(", "));
        }
    }
    println!("\nConfidence\n  {}", report.confidence);
}
