use std::{collections::BTreeSet, path::PathBuf};

use anyhow::Result;
use serde::Serialize;
use walkdir::WalkDir;

use crate::config::settings::ScannerConfig;

use super::{detector::detect_project, ignore::should_ignore_directory, project::ProjectInfo};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ScanEntry {
    pub path: PathBuf,
    pub is_directory: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanReport {
    pub project: ProjectInfo,
    pub files_inspected: usize,
    pub rust_source_files: usize,
    pub configuration_files: usize,
    pub knowledge_documents: usize,
    pub large_files_skipped: usize,
    pub ignored_directories: Vec<String>,
    pub entries: Vec<ScanEntry>,
    pub warnings: Vec<String>,
}

pub fn scan_project(path: &std::path::Path, config: &ScannerConfig) -> Result<ScanReport> {
    let project = detect_project(path)?;
    let mut report = ScanReport {
        project,
        files_inspected: 0,
        rust_source_files: 0,
        configuration_files: 0,
        knowledge_documents: 0,
        large_files_skipped: 0,
        ignored_directories: Vec::new(),
        entries: Vec::new(),
        warnings: Vec::new(),
    };
    let mut ignored = BTreeSet::new();
    let max_bytes = config.max_file_size_kb.saturating_mul(1024);
    let root = report.project.root.clone();

    let walker = WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            if entry.depth() == 0 || !entry.file_type().is_dir() {
                return true;
            }
            if should_ignore_directory(entry.path(), config.ignore_hidden) {
                if let Some(name) = entry.file_name().to_str() {
                    ignored.insert(format!("{name}/"));
                }
                false
            } else {
                true
            }
        });

    for result in walker {
        let entry = match result {
            Ok(entry) => entry,
            Err(error) => {
                report.warnings.push(error.to_string());
                continue;
            }
        };
        if entry.depth() == 0 {
            continue;
        }
        let relative = match entry.path().strip_prefix(&root) {
            Ok(path) => path.to_path_buf(),
            Err(_) => continue,
        };
        if config.ignore_hidden
            && relative
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with('.'))
        {
            continue;
        }
        report.entries.push(ScanEntry {
            path: relative.clone(),
            is_directory: entry.file_type().is_dir(),
        });
        if !entry.file_type().is_file() {
            continue;
        }
        let size = match entry.metadata() {
            Ok(metadata) => metadata.len(),
            Err(error) => {
                report
                    .warnings
                    .push(format!("{}: {error}", relative.display()));
                continue;
            }
        };
        if size > max_bytes {
            report.large_files_skipped += 1;
            continue;
        }
        report.files_inspected += 1;
        if relative
            .extension()
            .and_then(|extension| extension.to_str())
            == Some("rs")
        {
            report.rust_source_files += 1;
        }
        if is_configuration_file(&relative) {
            report.configuration_files += 1;
        }
        if relative
            .extension()
            .and_then(|extension| extension.to_str())
            == Some("md")
            && relative
                .components()
                .any(|component| component.as_os_str() == "knowledge")
        {
            report.knowledge_documents += 1;
        }
    }

    report.ignored_directories = ignored.into_iter().collect();
    report
        .entries
        .sort_by(|left, right| left.path.cmp(&right.path));
    Ok(report)
}

fn is_configuration_file(path: &std::path::Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    matches!(
        name,
        "Cargo.toml"
            | "package.json"
            | "pyproject.toml"
            | "requirements.txt"
            | "go.mod"
            | "Dockerfile"
            | "docker-compose.yml"
            | "docker-compose.yaml"
            | "compose.yml"
            | "compose.yaml"
            | "tsconfig.json"
    ) || name.ends_with(".config.js")
        || name.ends_with(".config.ts")
}

#[cfg(test)]
mod tests {
    use std::{fs, time::SystemTime};

    use anyhow::Context;

    use super::*;

    fn project_dir() -> Result<PathBuf> {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let path = std::env::temp_dir().join(format!("lbc-scan-{}-{nonce}", std::process::id()));
        fs::create_dir_all(path.join("src"))?;
        Ok(path)
    }

    #[test]
    fn ignores_generated_directories_and_large_files() -> Result<()> {
        let root = project_dir()?;
        fs::write(root.join("Cargo.toml"), "[package]")?;
        fs::write(root.join("src/main.rs"), "fn main() {}")?;
        fs::create_dir(root.join("target"))?;
        fs::write(root.join("target/generated.rs"), "generated")?;
        fs::write(root.join(".env"), "SECRET=do-not-scan")?;
        fs::write(root.join("large.log"), vec![0_u8; 2048])?;

        let report = scan_project(
            &root,
            &ScannerConfig {
                max_file_size_kb: 1,
                ignore_hidden: true,
            },
        )?;
        assert_eq!(report.rust_source_files, 1);
        assert_eq!(report.large_files_skipped, 1);
        assert_eq!(report.ignored_directories, ["target/"]);
        assert!(
            !report
                .entries
                .iter()
                .any(|entry| entry.path.starts_with("target"))
        );
        assert!(
            !report
                .entries
                .iter()
                .any(|entry| entry.path.as_os_str() == ".env")
        );
        fs::remove_dir_all(&root)
            .with_context(|| format!("failed to remove {}", root.display()))?;
        Ok(())
    }
}
