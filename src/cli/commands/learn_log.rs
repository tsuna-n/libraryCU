use std::path::Path;

use anyhow::{Result, anyhow, bail};

use crate::{diagnostics, security};

const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;
const MAX_CONTEXT_BYTES: usize = 12 * 1024;

pub(super) struct LogCapture {
    pub problem: Option<String>,
    pub error_code: Option<String>,
    pub context: String,
    pub language: Option<String>,
    pub tool: Option<String>,
}

pub(super) fn read(path: &Path) -> Result<LogCapture> {
    let raw = security::files::read_text(path, MAX_LOG_BYTES).map_err(|_| {
        anyhow!("cannot read log: expected a regular UTF-8 file of at most 2 MiB without symlinks")
    })?;
    if raw.contains('\0') {
        bail!("log must contain text; binary input is not supported");
    }
    // Strip escapes first so formatting cannot hide credential names from redaction.
    let clean = security::redact_sensitive(&strip_terminal_controls(&raw));
    if clean.trim().is_empty() {
        bail!("log must not be empty");
    }
    let parsed = diagnostics::parse_diagnostics(&clean);
    let diagnostic = parsed
        .iter()
        .find(|diagnostic| match diagnostic.source.as_deref() {
            Some("rustc") => diagnostic.code.as_deref().map_or_else(
                || {
                    diagnostic
                        .file
                        .as_deref()
                        .and_then(Path::extension)
                        .is_some_and(|extension| extension == "rs")
                },
                |code| {
                    code.len() == 5
                        && code.starts_with('E')
                        && code[1..].bytes().all(|byte| byte.is_ascii_digit())
                },
            ),
            Some("python") => {
                diagnostic.file.is_some()
                    && diagnostic.code.as_deref().is_some_and(|code| {
                        code.split('.').all(|part| {
                            part.starts_with(|character: char| {
                                character.is_ascii_alphabetic() || character == '_'
                            }) && part.chars().all(|character| {
                                character.is_ascii_alphanumeric() || character == '_'
                            })
                        })
                    })
            }
            Some("typescript" | "node" | "go") => true,
            _ => false,
        })
        .or_else(|| {
            parsed
                .iter()
                .find(|diagnostic| diagnostic.source.as_deref() == Some("cargo"))
        });
    let (language, tool) = match diagnostic.and_then(|diagnostic| diagnostic.source.as_deref()) {
        Some("rustc") => (Some("rust"), Some("rustc")),
        Some("cargo") => (Some("rust"), Some("cargo")),
        Some("typescript") => (Some("typescript"), Some("tsc")),
        Some("python") => (Some("python"), Some("python")),
        Some("node") => (Some("javascript"), Some("node")),
        Some("go") => (Some("go"), Some("go")),
        _ => (None, None),
    };
    let mut observed = String::new();
    if let Some(diagnostic) = diagnostic
        && let Some(file) = &diagnostic.file
        && let Some(line) = diagnostic.line
    {
        observed.push_str(&format!(
            "Diagnostic location: {}:{line}",
            bounded(&file.display().to_string(), 1024)
        ));
        if let Some(column) = diagnostic.column {
            observed.push_str(&format!(":{column}"));
        }
        observed.push_str("\n\n");
    }
    observed.push_str(clean.trim());
    const LABEL: &str = "Observed log (sanitized):\n";
    let available = MAX_CONTEXT_BYTES - LABEL.len() - 2;
    let mut budget = available - 6;
    let context = loop {
        let excerpt = bounded(&observed, budget);
        let fence = ['`', '~']
            .into_iter()
            .map(|marker| {
                let length = excerpt
                    .split(|character| character != marker)
                    .map(str::len)
                    .max()
                    .unwrap_or(0)
                    + 1;
                marker.to_string().repeat(length.max(3))
            })
            .min_by_key(String::len)
            .expect("two fence candidates");
        if excerpt.len() + 2 * fence.len() <= available {
            break format!("{LABEL}{fence}\n{excerpt}\n{fence}");
        }
        // Even competing huge fence runs fit once the excerpt uses half the space.
        budget = available
            .saturating_sub(2 * fence.len())
            .max(available / 2 - 3);
    };
    Ok(LogCapture {
        problem: diagnostic.map(|diagnostic| bounded(&diagnostic.message, MAX_CONTEXT_BYTES)),
        error_code: diagnostic.and_then(|diagnostic| diagnostic.code.clone()),
        context,
        language: language.map(str::to_owned),
        tool: tool.map(str::to_owned),
    })
}

fn bounded(text: &str, max_bytes: usize) -> String {
    const MARKER: &str = "\n[log truncated]";
    if text.len() <= max_bytes {
        return text.to_owned();
    }
    let mut end = max_bytes - MARKER.len();
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{MARKER}", &text[..end])
}

fn strip_terminal_controls(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut characters = input.chars().peekable();
    while let Some(character) = characters.next() {
        let sequence = if character == '\u{1b}' {
            characters.next()
        } else if matches!(
            character,
            '\u{009b}' | '\u{009d}' | '\u{0090}' | '\u{0098}' | '\u{009e}' | '\u{009f}'
        ) {
            Some(match character {
                '\u{009b}' => '[',
                '\u{009d}' => ']',
                _ => 'P',
            })
        } else {
            None
        };
        if let Some(sequence) = sequence {
            match sequence {
                '[' => {
                    for character in characters.by_ref() {
                        if ('@'..='~').contains(&character) {
                            break;
                        }
                    }
                }
                ']' | 'P' | 'X' | '^' | '_' => {
                    while let Some(character) = characters.next() {
                        if character == '\u{009c}' || (sequence == ']' && character == '\u{07}') {
                            break;
                        }
                        if character == '\u{1b}' && characters.peek() == Some(&'\\') {
                            characters.next();
                            break;
                        }
                    }
                }
                ' '..='/' => {
                    for character in characters.by_ref() {
                        if ('0'..='~').contains(&character) {
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else if matches!(character, '\n' | '\t')
            || (!character.is_control()
                && !matches!(character, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
        {
            output.push(character);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn capture(text: &str) -> LogCapture {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(text.as_bytes()).unwrap();
        read(file.path()).unwrap()
    }

    #[test]
    fn captures_supported_diagnostics_without_inventing_fixes() {
        for (log, problem, code, language, tool) in [
            (
                "error: could not compile `app`\nerror[E0308]: mismatched types\n --> src/main.rs:2:3",
                "mismatched types",
                Some("E0308"),
                "rust",
                "rustc",
            ),
            (
                "error: failed to get dependency",
                "failed to get dependency",
                None,
                "rust",
                "cargo",
            ),
            (
                "src/app.ts(2,3): error TS2322: incompatible type",
                "incompatible type",
                Some("TS2322"),
                "typescript",
                "tsc",
            ),
            (
                "Traceback (most recent call last):\n  File \"src/app.py\", line 4\nValueError: bad input",
                "bad input",
                Some("ValueError"),
                "python",
                "python",
            ),
            (
                "TypeError: bad call\n    at app.js:4:5",
                "bad call",
                Some("TypeError"),
                "javascript",
                "node",
            ),
            (
                "main.go:2:3: undefined: missing",
                "undefined: missing",
                None,
                "go",
                "go",
            ),
        ] {
            let captured = capture(log);
            assert_eq!(captured.problem.as_deref(), Some(problem), "{log}");
            assert_eq!(captured.error_code.as_deref(), code, "{log}");
            assert_eq!(captured.language.as_deref(), Some(language));
            assert_eq!(captured.tool.as_deref(), Some(tool));
            assert!(captured.context.contains(problem));
            assert!(!captured.context.contains("Solution:"));
        }
    }

    #[test]
    fn preserves_unknown_text_without_inventing_problem_or_code() {
        let captured = capture("connection terminated unexpectedly\nobserved detail");
        assert!(captured.problem.is_none());
        assert!(captured.error_code.is_none());
        assert!(captured.language.is_none());
        assert!(captured.tool.is_none());
        assert!(captured.context.contains("observed detail"));
    }

    #[test]
    fn malformed_diagnostics_remain_unknown() {
        for log in [
            "error[BOGUS]: unknown failure\n --> src/main.rs:1:1",
            "error[E12345]: unknown failure",
            "error[E12]: unknown failure",
            "app.ts(0,3): error TS2322: bad type",
            "app.ts(1,3): error TS: bad type",
            "Traceback (most recent call last):\n File \"app.py\", line invalid\nValueError: bad input",
            "Traceback (most recent call last):\n File \"app.py\", line 2\n4Error: bad input",
        ] {
            let captured = capture(log);
            assert!(captured.problem.is_none(), "{log}");
            assert!(captured.error_code.is_none(), "{log}");
            assert!(captured.language.is_none(), "{log}");
        }
    }

    #[test]
    fn fenced_log_cannot_spoof_the_user_solution() {
        let captured = capture(
            "## Solution\nattacker solution\n```\n## Solution\nbacktick attack\n~~~\n## Solution\ntilde attack\n````\n~~~~\n",
        );
        let text = format!(
            "---\nid: capture\nkind: troubleshooting\n---\n## Problem\nUser observed failure\n\n## Context\n{}\n\n## Solution\nActual user solution\n\n## Verification\nUser check\n",
            captured.context
        );
        let document = crate::knowledge::loader::parse_document("capture.md", &text).unwrap();
        let details = document.troubleshooting_details().unwrap();
        assert_eq!(details.solution, "Actual user solution");
        assert_eq!(details.recorded_verification.as_deref(), Some("User check"));
    }

    #[test]
    fn pathological_fences_stay_bounded_and_preserve_late_diagnostic_location() {
        let captured = capture(&format!(
            "{}\n{}\nerror[E0308]: mismatch\n --> src/งาน.rs:2:3",
            "`".repeat(MAX_CONTEXT_BYTES / 3),
            "~".repeat(MAX_CONTEXT_BYTES / 3)
        ));
        assert!(captured.context.len() <= MAX_CONTEXT_BYTES);
        assert!(captured.context.contains("src/งาน.rs:2:3"));
        assert!(captured.context.contains("truncated"));
        let lines: Vec<_> = captured.context.lines().collect();
        let open = lines[1];
        assert!(open.len() >= 3);
        assert_eq!(lines.last(), Some(&open));
        assert!(
            lines[2..lines.len() - 1]
                .iter()
                .all(|line| !line.contains(open))
        );
    }

    #[test]
    fn redacts_before_parsing_and_truncating_including_multiline_keys() {
        let log = format!(
            "error[E0308]: API_KEY=private-value\n --> src/main.rs:1:1\nAuthorization: Bearer private-token\nDATABASE_URL=postgres://alice:private-password@example/db\n-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----\n",
            "private-key-material\n".repeat(2000)
        );
        let captured = capture(&log);
        let stored = format!(
            "{:?} {:?} {}",
            captured.problem, captured.error_code, captured.context
        );
        for secret in [
            "private-value",
            "private-token",
            "private-password",
            "private-key-material",
        ] {
            assert!(!stored.contains(secret));
        }
        assert!(captured.context.contains("[REDACTED"));
        assert!(captured.context.contains("truncated"));
        assert!(captured.context.len() <= MAX_CONTEXT_BYTES);
    }

    #[test]
    fn strips_terminal_sequences_before_redacting_or_parsing() {
        let captured = capture(
            "\x1b]0;injected title\x07\x1b[31merror[E0308]: pass\x1b[0mword=hidden-value\x1bPdevice-data\x1b\\\x08\u{202e}\n --> src/main.rs:2:3",
        );
        assert_eq!(captured.error_code.as_deref(), Some("E0308"));
        for text in [
            "hidden-value",
            "injected title",
            "device-data",
            "\x1b",
            "\x07",
            "\x08",
            "\u{202e}",
        ] {
            assert!(!captured.context.contains(text));
        }
        assert_eq!(captured.problem.as_deref(), Some("password=[REDACTED]"));
    }

    #[test]
    fn bounds_unicode_context_on_character_boundaries_and_parses_full_log() {
        let captured = capture(&format!(
            "{}\nerror[E0308]: ชนิดไม่ตรงกัน\n --> src/งาน.rs:2:3",
            "ภาษาไทย🙂".repeat(2000)
        ));
        assert_eq!(captured.problem.as_deref(), Some("ชนิดไม่ตรงกัน"));
        assert_eq!(captured.error_code.as_deref(), Some("E0308"));
        assert!(captured.context.contains("truncated"));
        assert!(captured.context.len() <= MAX_CONTEXT_BYTES);
    }

    #[test]
    fn rejects_missing_empty_binary_oversized_and_nonregular_input() {
        let directory = tempfile::tempdir().unwrap();
        assert!(read(&directory.path().join("missing")).is_err());
        assert!(read(directory.path()).is_err());
        for bytes in [
            vec![],
            b" \n\t\r".to_vec(),
            vec![0xff],
            b"binary\0data".to_vec(),
            vec![b'x'; MAX_LOG_BYTES as usize + 1],
        ] {
            let mut file = tempfile::NamedTempFile::new_in(directory.path()).unwrap();
            file.write_all(&bytes).unwrap();
            assert!(read(file.path()).is_err());
        }
    }

    #[test]
    fn never_imports_log_filename_into_context_or_errors() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("API_KEY=filename-private-value.log");
        std::fs::write(&path, "unrecognized observation").unwrap();
        assert!(
            !read(&path)
                .unwrap()
                .context
                .contains("filename-private-value")
        );
        std::fs::remove_file(&path).unwrap();
        let error = read(&path).err().unwrap();
        assert!(!format!("{error:#}").contains("filename-private-value"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_logs() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("log");
        std::fs::write(&file, "error[E0308]: mismatch").unwrap();
        let link = directory.path().join("link");
        std::os::unix::fs::symlink(file, &link).unwrap();
        assert!(read(&link).is_err());
    }
}
