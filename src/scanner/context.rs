use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use serde::Serialize;

use crate::{config::settings::ScannerConfig, diagnostics::Diagnostic, security};

const MAX_TOTAL_CHARS: usize = 12_000;

#[derive(Debug, Clone, Serialize)]
pub struct EvidenceExcerpt {
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
}

pub fn collect_diagnostic_evidence(
    root: &Path,
    diagnostic: &Diagnostic,
    config: &ScannerConfig,
) -> (Vec<EvidenceExcerpt>, Vec<String>) {
    let mut excerpts = Vec::new();
    let mut warnings = Vec::new();
    let Ok(root) = root.canonicalize() else {
        return (
            excerpts,
            vec!["project root could not be canonicalized".to_owned()],
        );
    };
    let mut candidates: Vec<(PathBuf, Option<u32>)> = Vec::new();
    if let Some(path) = &diagnostic.file {
        candidates.push((root.join(path), diagnostic.line));
    }
    for name in ["Cargo.toml", "package.json", "pyproject.toml", "go.mod"] {
        let path = root.join(name);
        if path.is_file() {
            candidates.push((path, None));
        }
    }
    let mut remaining = MAX_TOTAL_CHARS;
    for (path, line) in candidates {
        let relative = path.strip_prefix(&root).unwrap_or(&path).to_path_buf();
        let relative_display = relative.display().to_string();
        if relative
            .components()
            .any(|component| component == Component::ParentDir)
        {
            warnings.push(format!("excluded out-of-root evidence {relative_display}"));
            continue;
        }
        if should_exclude_relative(&relative, config.ignore_hidden) {
            warnings.push(format!("excluded ignored evidence {relative_display}"));
            continue;
        }
        if is_gitignored(&root, &relative) {
            warnings.push(format!("excluded .gitignore evidence {relative_display}"));
            continue;
        }
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            warnings.push(format!("evidence file {relative_display} is unavailable"));
            continue;
        };
        if metadata.file_type().is_symlink() {
            warnings.push(format!("excluded symlink evidence {relative_display}"));
            continue;
        }
        if metadata.len() > config.max_file_size_kb.saturating_mul(1024) {
            warnings.push(format!("excluded oversized evidence {relative_display}"));
            continue;
        }
        let Ok(canonical) = path.canonicalize() else {
            warnings.push(format!(
                "evidence file {relative_display} cannot be resolved"
            ));
            continue;
        };
        if !canonical.starts_with(&root) {
            warnings.push(format!("excluded out-of-root evidence {relative_display}"));
            continue;
        }
        let Ok(content) = fs::read_to_string(&canonical) else {
            warnings.push(format!(
                "evidence file {relative_display} is not readable text"
            ));
            continue;
        };
        let lines: Vec<_> = content.lines().collect();
        let (start, end) = if let Some(line) = line {
            let center = line.saturating_sub(1) as usize;
            (center.saturating_sub(3), (center + 4).min(lines.len()))
        } else {
            (0, lines.len().min(80))
        };
        if start >= end || remaining == 0 {
            continue;
        }
        let numbered = lines[start..end]
            .iter()
            .enumerate()
            .map(|(offset, value)| format!("{}: {}", start + offset + 1, value))
            .collect::<Vec<_>>()
            .join("\n");
        let redacted = security::redact_sensitive(&numbered);
        let bounded: String = redacted.chars().take(remaining).collect();
        remaining = remaining.saturating_sub(bounded.chars().count());
        excerpts.push(EvidenceExcerpt {
            path: relative_display,
            start_line: start + 1,
            end_line: start + bounded.lines().count(),
            content: bounded,
        });
    }
    (excerpts, warnings)
}

fn should_exclude_relative(path: &Path, ignore_hidden: bool) -> bool {
    path.components().any(|component| {
        let Component::Normal(name) = component else {
            return false;
        };
        let path = Path::new(name);
        crate::scanner::ignore::should_ignore_directory(path, ignore_hidden)
    })
}

/// A deliberately small subset for common exact paths and ignored directories.
/// Complex gitignore syntax remains scanner metadata rather than being guessed.
fn is_gitignored(root: &Path, relative: &Path) -> bool {
    let Ok(content) = fs::read_to_string(root.join(".gitignore")) else {
        return false;
    };
    let candidate = relative.to_string_lossy().replace('\\', "/");
    let mut ignored = false;
    for line in content.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (negated, pattern) = line
            .strip_prefix('!')
            .map_or((false, line), |pattern| (true, pattern));
        let directory = pattern.ends_with('/');
        let pattern = pattern.trim_start_matches('/').trim_end_matches('/');
        let matches = if pattern.contains('/') {
            glob_matches(pattern, &candidate)
                || (directory && candidate.starts_with(&format!("{pattern}/")))
        } else {
            candidate
                .split('/')
                .any(|component| glob_matches(pattern, component))
        };
        if matches {
            ignored = !negated;
        }
    }
    ignored
}

fn glob_matches(pattern: &str, value: &str) -> bool {
    let pattern: Vec<_> = pattern.chars().collect();
    let value: Vec<_> = value.chars().collect();
    let mut table = vec![vec![false; value.len() + 1]; pattern.len() + 1];
    table[0][0] = true;
    for index in 0..pattern.len() {
        if pattern[index] == '*' {
            table[index + 1][0] = table[index][0];
        }
        for position in 0..value.len() {
            table[index + 1][position + 1] = match pattern[index] {
                '*' => table[index][position + 1] || table[index + 1][position],
                '?' => table[index][position],
                character => table[index][position] && character == value[position],
            };
        }
    }
    table[pattern.len()][value.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_root() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        std::env::temp_dir().join(format!("lbc-context-{}-{nonce}", std::process::id()))
    }
    #[test]
    fn path_escape_is_excluded() {
        let diagnostic = Diagnostic {
            source: None,
            code: None,
            message: "x".to_owned(),
            file: Some(PathBuf::from("../outside")),
            line: Some(1),
            column: None,
        };
        let (evidence, warnings) = collect_diagnostic_evidence(
            Path::new(env!("CARGO_MANIFEST_DIR")),
            &diagnostic,
            &ScannerConfig::default(),
        );
        assert!(evidence.iter().all(|item| item.path != "../outside"));
        assert!(!warnings.is_empty());
    }

    #[test]
    fn gitignored_and_oversized_diagnostic_files_are_excluded() {
        let root = temporary_root();
        fs::create_dir_all(root.join("ignored")).unwrap();
        fs::write(root.join(".gitignore"), "ignored/*.rs\n").unwrap();
        fs::write(root.join("ignored/source.rs"), "secret source").unwrap();
        let ignored = Diagnostic {
            source: None,
            code: None,
            message: "x".to_owned(),
            file: Some(PathBuf::from("ignored/source.rs")),
            line: Some(1),
            column: None,
        };
        let (evidence, warnings) =
            collect_diagnostic_evidence(&root, &ignored, &ScannerConfig::default());
        assert!(evidence.is_empty());
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains(".gitignore"))
        );

        fs::write(root.join("large.rs"), vec![b'x'; 2048]).unwrap();
        let large = Diagnostic {
            file: Some(PathBuf::from("large.rs")),
            ..ignored
        };
        let (evidence, warnings) = collect_diagnostic_evidence(
            &root,
            &large,
            &ScannerConfig {
                max_file_size_kb: 1,
                ignore_hidden: true,
            },
        );
        assert!(evidence.is_empty());
        assert!(warnings.iter().any(|warning| warning.contains("oversized")));
        fs::remove_dir_all(root).unwrap();
    }
}
