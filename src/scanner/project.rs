use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct ProjectInfo {
    pub root: PathBuf,
    pub languages: Vec<String>,
    pub build_systems: Vec<String>,
    pub containers: Vec<String>,
    pub frameworks: Vec<String>,
    pub source_directories: Vec<PathBuf>,
    pub additional: Vec<String>,
}

impl ProjectInfo {
    pub fn stack_label(&self) -> String {
        let mut parts = self.languages.clone();
        parts.extend(self.build_systems.iter().cloned());
        if parts.is_empty() {
            "Unknown project".to_owned()
        } else {
            parts.join(" / ")
        }
    }
}
