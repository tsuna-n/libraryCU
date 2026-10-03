use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct KnowledgeMetadata {
    pub id: String,
    pub kind: Option<String>,
    /// A source-qualified ID intentionally replaced by this document.
    pub overrides: Option<String>,
    pub language: Option<String>,
    pub tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub framework: Option<String>,
    pub category: Option<String>,
    pub error_code: Option<String>,
    pub title: Option<String>,
    /// Optional Thai display title. `title` remains the canonical English title.
    pub title_th: Option<String>,
    pub tags: Vec<String>,
    pub keywords: Vec<String>,
    /// `unverified`, `user-reported`, or `recorded-check`.
    pub verification_status: Option<String>,
    /// UTC timestamps in seconds since the Unix epoch; absent in older notes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at_unix: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at_unix: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeDocument {
    pub metadata: KnowledgeMetadata,
    pub title: String,
    pub body: String,
    /// Backward-compatible, human-readable source locator.
    pub path: String,
    /// `builtin`, `user`, `project`, or `package:<name>`.
    pub source: String,
    pub source_id: String,
    pub kind: String,
    pub verification_status: String,
    pub writable: bool,
    pub effective: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overridden_by: Option<String>,
}

/// Optional views of ordinary Markdown sections, not a second storage format.
#[derive(Debug, Clone, Serialize)]
pub struct TroubleshootingDetails {
    pub problem: String,
    pub solution: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_verification: Option<String>,
}

impl KnowledgeDocument {
    pub fn qualified_id(&self) -> &str {
        &self.source_id
    }

    pub fn troubleshooting_details(&self) -> Option<TroubleshootingDetails> {
        if self.kind != "troubleshooting" {
            return None;
        }
        let body = crate::security::redact_sensitive(&self.body);
        let section = |name: &str| -> Option<String> {
            let heading = format!("## {name}");
            let mut active = false;
            let mut fence: Option<(char, usize)> = None;
            let mut lines = Vec::new();
            for line in body.lines() {
                let trimmed = line.trim_start();
                if fence.is_none() {
                    if trimmed.eq_ignore_ascii_case(&heading) {
                        active = true;
                        continue;
                    }
                    if active && (trimmed.starts_with("# ") || trimmed.starts_with("## ")) {
                        break;
                    }
                }
                if active {
                    lines.push(line);
                }
                if let Some(marker @ ('`' | '~')) = trimmed.chars().next() {
                    let length = trimmed.chars().take_while(|c| *c == marker).count();
                    if length >= 3 {
                        match fence {
                            None => fence = Some((marker, length)),
                            Some((open, count))
                                if marker == open
                                    && length >= count
                                    && trimmed[length..].trim().is_empty() =>
                            {
                                fence = None
                            }
                            _ => {}
                        }
                    }
                }
            }
            let text = lines.join("\n");
            let text = text.trim();
            if text.is_empty() {
                return None;
            }
            let mut bounded: String = text.chars().take(1_000).collect();
            if text.chars().count() > 1_000 {
                bounded.push('…');
            }
            Some(bounded)
        };
        Some(TroubleshootingDetails {
            problem: section("Problem")?,
            solution: section("Solution")?,
            recorded_verification: section("Verification"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::loader::parse_document;

    #[test]
    fn markdown_details_ignore_fenced_headings_redact_and_bound_sections() {
        let text = format!(
            "---\nid: capture\nkind: troubleshooting\n---\n## Problem\n\nA diagnostic example:\n````text\n## Solution\nNot the actual solution\n```\n````\n\n## Solution\n\npassword=fixture-secret\n{}\n\n## Verification\n\ncargo test\n",
            "borrow ".repeat(200)
        );
        let document = parse_document("capture.md", &text).unwrap();
        let details = document.troubleshooting_details().unwrap();
        assert!(details.problem.contains("Not the actual solution"));
        assert!(!details.solution.contains("fixture-secret"));
        assert!(details.solution.contains("borrow"));
        assert!(details.solution.chars().count() <= 1_001);
        assert_eq!(details.recorded_verification.as_deref(), Some("cargo test"));
        let plain = parse_document("old.md", "---\nid: old\n---\nPlain older Markdown.").unwrap();
        assert!(plain.troubleshooting_details().is_none());
    }
}
