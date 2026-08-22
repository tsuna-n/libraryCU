use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct KnowledgeMetadata {
    pub id: String,
    pub language: Option<String>,
    pub category: Option<String>,
    pub error_code: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeDocument {
    pub metadata: KnowledgeMetadata,
    pub title: String,
    pub body: String,
    pub path: String,
}
