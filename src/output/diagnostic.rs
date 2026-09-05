use crate::diagnostics::ExplanationReport;

pub fn print_explanation(report: &ExplanationReport, verbose: bool) {
    print_explanation_language(report, verbose, "en");
}

pub fn print_explanation_language(report: &ExplanationReport, verbose: bool, language: &str) {
    if language == "th" {
        return print_explanation_thai(report, verbose);
    }
    let code = report.diagnostic.code.as_deref().unwrap_or("Unknown error");
    println!("libraryCube diagnostic\n");
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
                println!(
                    "  {} [{}] ({}, {}; status: {})\n    {}",
                    item.title,
                    item.source_id,
                    item.path,
                    item.match_reason,
                    item.verification_status,
                    item.excerpt
                );
            } else {
                println!(
                    "  {} [{}; status: {}]\n    {}",
                    item.title, item.source_id, item.verification_status, item.excerpt
                );
            }
        }
    } else {
        println!("\nLocal knowledge\n  No exact match found.");
    }
    if verbose {
        println!("\nProject context");
        println!("  Eligible files inventoried: {}", report.files_inspected);
        println!(
            "  File contents used as evidence: {}",
            report.project_evidence.len()
        );
        if !report.project.frameworks.is_empty() {
            println!("  Frameworks: {}", report.project.frameworks.join(", "));
        }
    }
    if !report.warnings.is_empty() {
        println!("\nWarnings");
        for warning in &report.warnings {
            println!("  - {warning}");
        }
    }
    if let Some(error) = &report.ai_error {
        println!("\nAI status\n  unavailable: {error}");
    }
    if let Some(ai) = &report.ai {
        println!("\nAI analysis ({} / {})", ai.provider, ai.model);
        println!("  {}", ai.analysis.replace('\n', "\n  "));
        println!("\nAI confidence\n  {}", ai.confidence);
    }
    println!("\nConfidence\n  {}", report.confidence);
}

fn print_explanation_thai(report: &ExplanationReport, verbose: bool) {
    let code = report
        .diagnostic
        .code
        .as_deref()
        .unwrap_or("ข้อผิดพลาดไม่ทราบชนิด");
    println!("การวิเคราะห์จาก libraryCube\n");
    println!("✗ {code} - {}\n", report.diagnostic.message);
    println!("โปรเจกต์\n  {}\n", report.project.stack_label());
    println!("หลักฐาน");
    for evidence in &report.evidence {
        println!("  - {evidence}");
    }
    println!("\nสาเหตุ\n  {}", report.cause);
    if !report.suggested_fixes.is_empty() {
        println!("\nคำแนะนำ (ยังไม่ได้ยืนยันผลกับโปรเจกต์นี้)");
        for (index, fix) in report.suggested_fixes.iter().enumerate() {
            println!("  {}. {fix}", index + 1);
        }
    }
    if !report.next_steps.is_empty() {
        println!("\nขั้นตอนตรวจสอบต่อ");
        for (index, step) in report.next_steps.iter().enumerate() {
            println!("  {}. {step}", index + 1);
        }
    }
    if !report.verification.is_empty() {
        println!("\nคำสั่งที่แนะนำให้ผู้ใช้ตรวจสอบเอง");
        for command in &report.verification {
            println!("  {command}");
        }
    }
    println!("\nแหล่งความรู้");
    if report.knowledge.is_empty() {
        println!("  ไม่พบรายการที่เกี่ยวข้องเพียงพอ");
    }
    for item in &report.knowledge {
        println!(
            "  {} [{}; สถานะบันทึก: {}]\n    {}",
            item.title, item.source_id, item.verification_status, item.excerpt
        );
    }
    if verbose {
        println!(
            "\nเนื้อหาไฟล์ที่ใช้เป็นหลักฐาน: {} รายการ",
            report.project_evidence.len()
        );
    }
    if let Some(ai) = &report.ai {
        println!(
            "\nคำอธิบายจาก AI ({} / {})\n  {}",
            ai.provider,
            ai.model,
            ai.analysis.replace('\n', "\n  ")
        );
    }
    if let Some(error) = &report.ai_error {
        println!("\nสถานะ AI\n  ใช้งานไม่ได้: {error}");
    }
    println!("\nระดับความมั่นใจ\n  {}", report.confidence);
}
