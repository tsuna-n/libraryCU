use std::path::Path;

use anyhow::Result;
use serde::Serialize;

use crate::{
    config::settings::{OutputConfig, ScannerConfig},
    knowledge::{self, SearchResult},
    scanner::EvidenceExcerpt,
    security,
};

const MAX_PASSAGES: usize = 4;

#[derive(Debug, Clone, Serialize)]
pub struct AnswerPassage {
    pub source_id: String,
    pub title: String,
    pub source_locator: String,
    pub excerpt: String,
    pub match_reason: String,
    pub ranking_reasons: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<knowledge::document::TroubleshootingDetails>,
    pub score: u32,
    pub verification_status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnswerReport {
    pub question: String,
    pub language: String,
    pub answer_status: String,
    pub passages: Vec<AnswerPassage>,
    pub offline_answer: String,
    pub project_evidence: Vec<EvidenceExcerpt>,
    pub warnings: Vec<String>,
}

pub fn answer(
    question: &str,
    project: &Path,
    output: &OutputConfig,
    scanner: &ScannerConfig,
) -> Result<AnswerReport> {
    answer_with_scope(
        question,
        project,
        output,
        scanner,
        knowledge::retrieval::RetrievalScope::All,
    )
}

pub fn answer_with_scope(
    question: &str,
    project: &Path,
    output: &OutputConfig,
    scanner: &ScannerConfig,
    scope: knowledge::retrieval::RetrievalScope,
) -> Result<AnswerReport> {
    knowledge::retrieval::validate_query(question)?;
    let redacted_question = security::redact_sensitive(question);
    let question = redacted_question.as_str();
    let language = choose_language(&output.language, question);
    let retrieved = knowledge::retrieval::retrieve_with_scope(project, question, &language, scope)?;
    let mut warnings: Vec<_> = retrieved
        .invalid
        .iter()
        .map(|invalid| security::redact_sensitive(&format!("{}: {}", invalid.path, invalid.error)))
        .collect();
    let (project_evidence, evidence_warnings) = crate::diagnostics::parse_primary(question)
        .filter(|diagnostic| diagnostic.file.is_some())
        .map(|diagnostic| {
            crate::scanner::collect_diagnostic_evidence(project, &diagnostic, scanner)
        })
        .unwrap_or_default();
    warnings.extend(evidence_warnings);
    let mut selected: Vec<_> = knowledge::retrieval::supporting_results(&retrieved.results)
        .take(MAX_PASSAGES)
        .collect();
    if selected.is_empty()
        && let Some(fallback) = retrieved.results.iter().find(|result| {
            result.match_reason == knowledge::index::PYTHON_ERROR_FALLBACK_REASON
                && !result.excerpt.is_empty()
        })
    {
        selected.push(fallback);
    }
    let passages = selected
        .iter()
        .map(|result| AnswerPassage {
            source_id: security::redact_sensitive(&result.document.source_id),
            title: security::redact_sensitive(&result.document.title),
            source_locator: security::redact_sensitive(&result.document.path),
            excerpt: result.excerpt.clone(),
            match_reason: result.match_reason.clone(),
            ranking_reasons: result.ranking_reasons.clone(),
            details: result.document.troubleshooting_details(),
            score: result.score,
            verification_status: verification_label(
                &result.document.verification_status,
                &language,
            ),
        })
        .collect::<Vec<_>>();
    let (answer_status, offline_answer) = if passages.is_empty() {
        (
            "no_adequate_match".to_owned(),
            if language == "th" {
                format!(
                    "ยังไม่พบความรู้ที่เกี่ยวข้องเพียงพอสำหรับ: {question}\n\nขั้นตอนถัดไป: ตรวจคำสำคัญหรือข้อความผิดพลาดให้ครบ แล้วเพิ่มบันทึกที่ตรวจสอบแหล่งที่มาได้"
                )
            } else {
                format!(
                    "No sufficiently relevant local knowledge was found for: {question}\n\nNext steps: include the complete error or distinctive keywords, then add a source-checked note if needed."
                )
            },
        )
    } else {
        let general = selected
            .iter()
            .all(|result| result.match_reason == knowledge::index::PYTHON_ERROR_FALLBACK_REASON);
        let mut text = if general && language == "th" {
            "แนวทางตรวจสอบ Python ทั่วไป; ยังไม่พบหลักฐานเพียงพอที่จะระบุวิธีแก้ข้อผิดพลาดนี้:\n".to_owned()
        } else if general {
            "General Python investigation guidance; insufficient evidence for a specific fix:\n"
                .to_owned()
        } else if language == "th" {
            "คำแนะนำจากความรู้ที่ค้นพบ:\n".to_owned()
        } else {
            "Retrieved guidance:\n".to_owned()
        };
        for passage in &passages {
            if language == "th" {
                text.push_str(&format!(
                    "\n[{}] {}\nเหตุผลที่ตรงกัน: {}\n{}\nสถานะ: {}\n",
                    passage.source_id,
                    passage.title,
                    passage.ranking_reasons.join(" + "),
                    passage.excerpt,
                    passage.verification_status
                ));
            } else {
                text.push_str(&format!(
                    "\n[{}] {}\nMatch: {}\n{}\nStatus: {}\n",
                    passage.source_id,
                    passage.title,
                    passage.ranking_reasons.join(" + "),
                    passage.excerpt,
                    passage.verification_status
                ));
            }
            if let Some(details) = &passage.details {
                let (problem, solution, check) = if language == "th" {
                    ("ปัญหาที่บันทึก", "วิธีแก้ที่เคยใช้", "การตรวจสอบที่บันทึกไว้ (ไม่ได้ตรวจซ้ำ)")
                } else {
                    (
                        "Recorded problem",
                        "Previous solution",
                        "Verification recorded (not rerun)",
                    )
                };
                text.push_str(&format!(
                    "{problem}:\n{}\n{solution}:\n{}\n",
                    details.problem, details.solution
                ));
                if let Some(verification) = &details.recorded_verification {
                    text.push_str(&format!("{check}:\n{verification}\n"));
                }
            }
        }
        (
            if general {
                "general_guidance"
            } else {
                "retrieved_guidance"
            }
            .to_owned(),
            text.trim_end().to_owned(),
        )
    };
    Ok(AnswerReport {
        question: question.to_owned(),
        language,
        answer_status,
        passages,
        offline_answer,
        project_evidence,
        warnings,
    })
}

fn verification_label(status: &str, language: &str) -> String {
    match (status, language) {
        ("user-reported", "th") => "ผู้เขียนบันทึกระบุว่าใช้งานได้; libraryCube ยังไม่ได้ตรวจซ้ำ".to_owned(),
        ("recorded-check", "th") => "บันทึกมีผลการตรวจที่บันทึกไว้; ไม่ใช่การตรวจใหม่ของคำสั่งนี้".to_owned(),
        (_, "th") => "คำแนะนำจากคลังความรู้; ยังไม่ได้ตรวจยืนยันกับโปรเจกต์นี้".to_owned(),
        ("user-reported", _) => "user-reported success; not rechecked by libraryCube".to_owned(),
        ("recorded-check", _) => {
            "the note records a check; this command did not rerun it".to_owned()
        }
        _ => "retrieved guidance; not verified against this project".to_owned(),
    }
}

pub fn is_adequate(result: &SearchResult) -> bool {
    knowledge::retrieval::is_adequate(result)
}

pub fn choose_language(setting: &str, input: &str) -> String {
    match setting {
        "th" => "th",
        "en" => "en",
        _ if input
            .chars()
            .any(|character| ('\u{0E00}'..='\u{0E7F}').contains(&character)) =>
        {
            "th"
        }
        _ => "en",
    }
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn auto_uses_thai_only_when_question_contains_thai() {
        assert_eq!(choose_language("auto", "แก้ปัญหาอย่างไร"), "th");
        assert_eq!(choose_language("auto", "how to fix"), "en");
    }
}
