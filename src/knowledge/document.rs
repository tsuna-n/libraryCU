use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct KnowledgeMetadata {
    pub id: String,
    pub kind: Option<String>,
    /// A source-qualified ID intentionally replaced by this document.
    pub overrides: Option<String>,
    pub language: Option<String>,
    pub tool: Option<String>,
    pub category: Option<String>,
    pub error_code: Option<String>,
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub keywords: Vec<String>,
    /// `unverified`, `user-reported`, or `recorded-check`.
    pub verification_status: Option<String>,
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

impl KnowledgeDocument {
    pub fn qualified_id(&self) -> &str {
        &self.source_id
    }
}
