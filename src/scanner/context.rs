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
        if is_gitignored_path(&root, &relative, false) {
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
        let Ok(content) = crate::security::files::read_text(
            &canonical,
            config.max_file_size_kb.saturating_mul(1024),
        ) else {
            warnings.push(format!(
                "evidence file {relative_display} is not readable text"
            ));
            continue;
        };
        // A selected line can lie inside a private key block whose marker is
        // outside the excerpt. Redact the complete file before slicing it.
        let content = security::redact_sensitive(&content);
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

pub(crate) fn is_gitignored(root: &Path, relative: &Path) -> bool {
    is_gitignored_path(root, relative, false)
}

pub(crate) fn is_gitignored_path(root: &Path, relative: &Path, is_dir: bool) -> bool {
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return true;
    }
    let components: Vec<_> = relative.components().collect();
    let mut prefix = PathBuf::new();
    for (index, component) in components.iter().enumerate() {
        prefix.push(component.as_os_str());
        let prefix_is_dir = index + 1 < components.len() || is_dir;
        if gitignore_match(root, &prefix, prefix_is_dir) {
            // Git cannot re-include a child of an ignored directory because
            // the directory is not traversed. Checking each prefix in order
            // preserves that rule for direct diagnostic paths too.
            return true;
        }
    }
    false
}

fn gitignore_match(root: &Path, relative: &Path, is_dir: bool) -> bool {
    let candidate = root.join(relative);
    let has_git_metadata = root.join(".git").exists();
    let mut ignored = false;

    if has_git_metadata {
        let (global, error) = ignore::gitignore::GitignoreBuilder::new(root).build_global();
        if error.is_some() {
            return true;
        }
        update_ignore_state(&mut ignored, global.matched(&candidate, is_dir));

        let exclude_file = root.join(".git/info/exclude");
        if exclude_file.is_file() {
            let mut builder = ignore::gitignore::GitignoreBuilder::new(root);
            if builder.add(&exclude_file).is_some() {
                return true;
            }
            let Ok(matcher) = builder.build() else {
                return true;
            };
            update_ignore_state(&mut ignored, matcher.matched(relative, is_dir));
        }
    }

    let mut directories = vec![root.to_path_buf()];
    let mut current = root.to_path_buf();
    let parent_components = relative.components().count().saturating_sub(1);
    for component in relative.components().take(parent_components) {
        current.push(component.as_os_str());
        directories.push(current.clone());
    }
    for directory in directories {
        let ignore_file = directory.join(".gitignore");
        if !ignore_file.is_file() {
            continue;
        }
        let mut builder = ignore::gitignore::GitignoreBuilder::new(&directory);
        if builder.add(&ignore_file).is_some() {
            return true;
        }
        let Ok(matcher) = builder.build() else {
            return true;
        };
        let relative_to_ignore = candidate.strip_prefix(&directory).unwrap_or(&candidate);
        update_ignore_state(&mut ignored, matcher.matched(relative_to_ignore, is_dir));
    }
    ignored
}

fn update_ignore_state<T>(ignored: &mut bool, matched: ignore::Match<T>) {
    if matched.is_ignore() {
        *ignored = true;
    } else if matched.is_whitelist() {
        *ignored = false;
    }
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

    #[test]
    fn gitignore_supports_recursive_patterns_classes_escapes_and_negation() {
        let root = temporary_root();
        fs::create_dir_all(root.join("logs/deep")).unwrap();
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(
            root.join(".gitignore"),
            "logs/**/[0-9].log\n\\#literal\n*.secret\n!public.secret\n",
        )
        .unwrap();
        fs::write(root.join("nested/.gitignore"), "*.tmp\n!keep.tmp\n").unwrap();

        assert!(is_gitignored(&root, Path::new("logs/deep/7.log")));
        assert!(is_gitignored(&root, Path::new("#literal")));
        assert!(is_gitignored(&root, Path::new("private.secret")));
        assert!(!is_gitignored(&root, Path::new("public.secret")));
        assert!(is_gitignored(&root, Path::new("nested/drop.tmp")));
        assert!(!is_gitignored(&root, Path::new("nested/keep.tmp")));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn gitignore_honors_repository_excludes_and_ignored_parent_rules() {
        let root = temporary_root();
        fs::create_dir_all(root.join(".git/info")).unwrap();
        fs::create_dir_all(root.join("ignored")).unwrap();
        fs::write(
            root.join(".git/info/exclude"),
            "local.cache\nvisible.cache\n",
        )
        .unwrap();
        fs::write(
            root.join(".gitignore"),
            "!visible.cache\nignored/\n!ignored/keep.txt\n",
        )
        .unwrap();

        assert!(is_gitignored(&root, Path::new("local.cache")));
        assert!(!is_gitignored(&root, Path::new("visible.cache")));
        assert!(is_gitignored(&root, Path::new("ignored/keep.txt")));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn excerpt_inside_private_key_block_retains_correct_line_bounds() {
        let root = temporary_root();
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("fixture.txt"),
            format!(
                "-----BEGIN PRIVATE KEY-----\n{}-----END PRIVATE KEY-----\n",
                "interior-key-material\n".repeat(20),
            ),
        )
        .unwrap();
        let diagnostic = Diagnostic {
            source: None,
            code: None,
            message: "fixture".into(),
            file: Some("fixture.txt".into()),
            line: Some(10),
            column: Some(1),
        };
        let (evidence, warnings) =
            collect_diagnostic_evidence(&root, &diagnostic, &ScannerConfig::default());
        assert!(warnings.is_empty());
        assert_eq!(evidence.len(), 1);
        assert_eq!((evidence[0].start_line, evidence[0].end_line), (7, 13));
        assert!(!evidence[0].content.contains("interior-key-material"));
        assert!(evidence[0].content.contains("10: [REDACTED PRIVATE KEY]"));
        fs::remove_dir_all(root).unwrap();
    }
}
