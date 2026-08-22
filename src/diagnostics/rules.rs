use std::{fs, path::Path};

use serde::Serialize;

use super::diagnostic::Diagnostic;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    VerifiedProject,
    KnownRule,
    Unknown,
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::VerifiedProject => "Verified from project files",
            Self::KnownRule => "Known diagnostic rule",
            Self::Unknown => "Unknown",
        };
        formatter.write_str(label)
    }
}

#[derive(Debug, Clone)]
pub struct RuleOutcome {
    pub evidence: Vec<String>,
    pub cause: String,
    pub suggested_fixes: Vec<String>,
    pub verification: Vec<String>,
    pub next_steps: Vec<String>,
    pub confidence: Confidence,
}

pub fn apply_rule(diagnostic: &Diagnostic, project_root: &Path, raw_input: &str) -> RuleOutcome {
    match diagnostic.code.as_deref() {
        Some("E0382") => moved_value_rule(diagnostic, raw_input),
        Some("E0432" | "E0433") => unresolved_import_rule(diagnostic, project_root),
        Some("E0499") => multiple_mutable_borrows_rule(diagnostic),
        _ => unknown_rule(diagnostic),
    }
}

fn moved_value_rule(diagnostic: &Diagnostic, raw_input: &str) -> RuleOutcome {
    let mut evidence = vec![format!("rustc reported: {}", diagnostic.message)];
    if raw_input.contains("value moved here") {
        evidence.push("The compiler trace marks where the value was moved.".to_owned());
    }
    RuleOutcome {
        evidence,
        cause: "A non-Copy value was moved to another owner and then used again. Passing values such as String by value transfers ownership.".to_owned(),
        suggested_fixes: vec![
            "Pass a reference (for example `&value` or `&str`) when the callee only needs to read the value.".to_owned(),
            "Clone the value only when the program genuinely needs a second owned value.".to_owned(),
        ],
        verification: vec!["cargo check".to_owned()],
        next_steps: Vec::new(),
        confidence: Confidence::KnownRule,
    }
}

fn unresolved_import_rule(diagnostic: &Diagnostic, project_root: &Path) -> RuleOutcome {
    let crate_name = extract_quoted_name(&diagnostic.message)
        .and_then(|name| name.split("::").next())
        .map(str::to_owned);
    let Some(crate_name) = crate_name.filter(|name| is_external_crate_candidate(name)) else {
        return RuleOutcome {
            evidence: vec![format!("rustc reported: {}", diagnostic.message)],
            cause: "Rust could not resolve the import path. The available evidence does not distinguish an external crate from a local module.".to_owned(),
            suggested_fixes: vec!["Check the import spelling, local module declarations, visibility, and enabled dependency features.".to_owned()],
            verification: vec!["cargo check".to_owned()],
            next_steps: Vec::new(),
            confidence: Confidence::KnownRule,
        };
    };

    let cargo_path = project_root.join("Cargo.toml");
    match dependency_is_declared(&cargo_path, &crate_name) {
        Some(false) => RuleOutcome {
            evidence: vec![
                format!("The diagnostic references the crate `{crate_name}`."),
                format!("{} does not declare `{crate_name}` as a dependency.", cargo_path.display()),
            ],
            cause: format!("The source uses `{crate_name}`, but the Cargo manifest does not declare that external crate."),
            suggested_fixes: vec![format!("cargo add {crate_name}")],
            verification: vec!["cargo check".to_owned()],
            next_steps: Vec::new(),
            confidence: Confidence::VerifiedProject,
        },
        Some(true) => RuleOutcome {
            evidence: vec![
                format!("The diagnostic references `{crate_name}`."),
                format!("{} already declares `{crate_name}`.", cargo_path.display()),
            ],
            cause: "The dependency exists, so the unresolved path is more likely caused by a disabled feature, version-specific API, spelling error, or local module path.".to_owned(),
            suggested_fixes: vec![
                format!("Inspect the `{crate_name}` entry and enabled features in Cargo.toml."),
                "Compare the import with the API exposed by the installed dependency version.".to_owned(),
            ],
            verification: vec!["cargo check".to_owned(), format!("cargo tree -i {crate_name}")],
            next_steps: Vec::new(),
            confidence: Confidence::KnownRule,
        },
        None => RuleOutcome {
            evidence: vec![format!("The diagnostic references `{crate_name}`.")],
            cause: "Rust could not resolve the crate or module, but LCU could not read a Cargo manifest to verify whether the dependency is missing.".to_owned(),
            suggested_fixes: vec!["Check Cargo.toml, import spelling, local module declarations, and enabled features.".to_owned()],
            verification: vec!["cargo check".to_owned()],
            next_steps: Vec::new(),
            confidence: Confidence::KnownRule,
        },
    }
}

fn multiple_mutable_borrows_rule(diagnostic: &Diagnostic) -> RuleOutcome {
    RuleOutcome {
        evidence: vec![format!("rustc reported: {}", diagnostic.message)],
        cause: "Two mutable borrows of the same value overlap. Rust permits only one active mutable reference to a value at a time.".to_owned(),
        suggested_fixes: vec![
            "End the first borrow before creating the second, often by narrowing its scope.".to_owned(),
            "When fields are disjoint, borrow or split those fields separately.".to_owned(),
        ],
        verification: vec!["cargo check".to_owned()],
        next_steps: Vec::new(),
        confidence: Confidence::KnownRule,
    }
}

fn unknown_rule(diagnostic: &Diagnostic) -> RuleOutcome {
    RuleOutcome {
        evidence: vec![format!("Detected message: {}", diagnostic.message)],
        cause: "LCU could not determine a verified root cause from the available error and project evidence.".to_owned(),
        suggested_fixes: Vec::new(),
        verification: Vec::new(),
        next_steps: vec![
            "Inspect the complete compiler or runtime stack trace.".to_owned(),
            "Identify the process or tool that produced the error.".to_owned(),
            "Run the failing command again with verbose logging.".to_owned(),
        ],
        confidence: Confidence::Unknown,
    }
}

fn extract_quoted_name(message: &str) -> Option<&str> {
    let (_, after_open) = message.split_once('`')?;
    let (name, _) = after_open.split_once('`')?;
    (!name.is_empty()).then_some(name)
}

fn is_external_crate_candidate(name: &str) -> bool {
    !matches!(name, "crate" | "self" | "super" | "std" | "core" | "alloc")
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}

fn dependency_is_declared(path: &Path, crate_name: &str) -> Option<bool> {
    let content = fs::read_to_string(path).ok()?;
    let manifest = toml::from_str::<toml::Value>(&content).ok()?;
    let candidates = [crate_name.to_owned(), crate_name.replace('_', "-")];
    let direct = ["dependencies", "dev-dependencies", "build-dependencies"]
        .iter()
        .any(|section| table_contains_any(&manifest, section, &candidates));
    let workspace = manifest
        .get("workspace")
        .is_some_and(|workspace| table_contains_any(workspace, "dependencies", &candidates));
    let target = manifest
        .get("target")
        .and_then(toml::Value::as_table)
        .is_some_and(|targets| {
            targets.values().any(|target| {
                ["dependencies", "dev-dependencies", "build-dependencies"]
                    .iter()
                    .any(|section| table_contains_any(target, section, &candidates))
            })
        });
    Some(direct || workspace || target)
}

fn table_contains_any(value: &toml::Value, table: &str, candidates: &[String]) -> bool {
    value
        .get(table)
        .and_then(toml::Value::as_table)
        .is_some_and(|dependencies| {
            candidates
                .iter()
                .any(|name| dependencies.contains_key(name))
        })
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use anyhow::Result;

    use super::*;

    fn project_without_dependency() -> Result<std::path::PathBuf> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let root = std::env::temp_dir().join(format!("lcu-rule-{}-{nonce}", std::process::id()));
        fs::create_dir_all(&root)?;
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )?;
        Ok(root)
    }

    #[test]
    fn verifies_a_missing_dependency_from_cargo_manifest() -> Result<()> {
        let root = project_without_dependency()?;
        let diagnostic = Diagnostic {
            source: Some("rustc".to_owned()),
            code: Some("E0432".to_owned()),
            message: "unresolved import `tokio`".to_owned(),
            file: None,
            line: None,
            column: None,
        };
        let outcome = apply_rule(&diagnostic, &root, "");
        assert_eq!(outcome.confidence, Confidence::VerifiedProject);
        assert_eq!(outcome.suggested_fixes, ["cargo add tokio"]);
        assert!(
            outcome
                .evidence
                .iter()
                .any(|line| line.contains("does not declare"))
        );
        fs::remove_dir_all(root)?;
        Ok(())
    }
}
