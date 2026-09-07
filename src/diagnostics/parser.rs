use std::path::PathBuf;

use super::diagnostic::Diagnostic;

pub fn parse_primary(input: &str) -> Option<Diagnostic> {
    parse_diagnostics(input).into_iter().next()
}

pub fn parse_diagnostics(input: &str) -> Vec<Diagnostic> {
    let clean = strip_ansi(input);
    let lines: Vec<_> = clean.lines().collect();
    let mut diagnostics = Vec::new();
    let mut index = 0;
    let mut python_traceback = false;
    let mut python_location = None;

    while index < lines.len() {
        let text = lines[index].trim();
        if let Some(diagnostic) = parse_compiler_line(text)
            .or_else(|| parse_node_error(text, lines.get(index + 1).copied()))
        {
            diagnostics.push(diagnostic);
            python_traceback = false;
            python_location = None;
            index += 1;
            continue;
        }
        if text == "Traceback (most recent call last):" {
            python_traceback = true;
            python_location = None;
            index += 1;
            continue;
        }
        if let Some(location) = parse_python_location(text) {
            python_traceback = true;
            python_location = Some(location);
            index += 1;
            continue;
        }
        if python_traceback && let Some((code, message)) = parse_python_exception(text) {
            let (file, line) = python_location
                .take()
                .map_or((None, None), |(file, line)| (Some(file), Some(line)));
            diagnostics.push(Diagnostic {
                source: Some("python".to_owned()),
                code: Some(code),
                message,
                file,
                line,
                column: None,
            });
            python_traceback = false;
            index += 1;
            continue;
        }
        let Some((code, message)) = parse_error_header(lines[index]) else {
            index += 1;
            continue;
        };
        python_traceback = false;
        python_location = None;
        let mut file = None;
        let mut line = None;
        let mut column = None;
        let mut cursor = index + 1;
        while cursor < lines.len()
            && parse_error_header(lines[cursor]).is_none()
            && lines[cursor].trim() != "Traceback (most recent call last):"
            && parse_python_location(lines[cursor].trim()).is_none()
            && parse_compiler_line(lines[cursor].trim()).is_none()
            && parse_node_error(lines[cursor].trim(), lines.get(cursor + 1).copied()).is_none()
        {
            if let Some(location) = parse_location(lines[cursor]) {
                file = Some(location.0);
                line = Some(location.1);
                column = Some(location.2);
                break;
            }
            cursor += 1;
        }
        let source = classify_source(code.as_deref(), file.as_deref(), &message);
        diagnostics.push(Diagnostic {
            source,
            code,
            message,
            file,
            line,
            column,
        });
        index = cursor.max(index + 1);
    }

    if diagnostics.is_empty()
        && let Some(message) = lines
            .iter()
            .map(|line| line.trim())
            .find(|line| !line.is_empty())
    {
        diagnostics.push(Diagnostic {
            source: None,
            code: None,
            message: message.to_owned(),
            file: None,
            line: None,
            column: None,
        });
    }

    diagnostics
}

fn parse_python_location(line: &str) -> Option<(PathBuf, u32)> {
    let rest = line.strip_prefix("File \"")?;
    let (file, rest) = rest.split_once("\", line ")?;
    let number = rest.split(',').next()?.trim().parse::<u32>().ok()?;
    if file.is_empty() || number == 0 || file.starts_with('<') {
        return None;
    }
    Some((PathBuf::from(file), number))
}

fn positive(value: &str) -> Option<u32> {
    value.parse().ok().filter(|value| *value > 0)
}

fn colon_location(value: &str) -> Option<(PathBuf, u32, u32)> {
    let mut pieces = value.rsplitn(3, ':');
    let column = positive(pieces.next()?)?;
    let line = positive(pieces.next()?)?;
    let file = pieces.next()?.trim();
    (!file.is_empty()).then(|| (file.into(), line, column))
}

fn parse_compiler_line(text: &str) -> Option<Diagnostic> {
    if let Some((location, rest)) = text.split_once("error TS") {
        let (code, message) = rest.split_once(':')?;
        if code.is_empty() || !code.bytes().all(|c| c.is_ascii_digit()) || message.trim().is_empty()
        {
            return None;
        }
        let location = location
            .trim()
            .trim_end_matches('-')
            .trim()
            .trim_end_matches(':');
        let (file, line, column) = if let Some(location) = location.strip_suffix(')') {
            let (file, coordinates) = location.rsplit_once('(')?;
            let (line, column) = coordinates.split_once(',')?;
            if file.is_empty() {
                return None;
            }
            (file.into(), positive(line)?, positive(column)?)
        } else {
            colon_location(location)?
        };
        return Some(Diagnostic {
            source: Some("typescript".into()),
            code: Some(format!("TS{code}")),
            message: message.trim().into(),
            file: Some(file),
            line: Some(line),
            column: Some(column),
        });
    }
    let (location, message) = text.split_once(": ")?;
    let (file, line, column) = if let Some((file, line, column)) = colon_location(location) {
        (file, line, Some(column))
    } else {
        let (file, line) = location.rsplit_once(':')?;
        (PathBuf::from(file), positive(line)?, None)
    };
    if file.extension()?.to_str()? != "go" || message.is_empty() {
        return None;
    }
    Some(Diagnostic {
        source: Some("go".into()),
        code: None,
        message: message.into(),
        file: Some(file),
        line: Some(line),
        column,
    })
}

fn parse_node_error(text: &str, next: Option<&str>) -> Option<Diagnostic> {
    let (header, message) = text.split_once(": ")?;
    let (kind, explicit_code) = if let Some((kind, code)) = header.split_once(" [") {
        (kind, Some(code.strip_suffix(']')?))
    } else {
        (header, None)
    };
    if !matches!(
        kind,
        "Error"
            | "TypeError"
            | "ReferenceError"
            | "SyntaxError"
            | "RangeError"
            | "URIError"
            | "EvalError"
    ) {
        return None;
    }
    let location = next.and_then(|next| {
        let frame = next.trim().strip_prefix("at ")?;
        let location = frame
            .strip_suffix(')')
            .and_then(|f| f.rsplit_once('(').map(|(_, loc)| loc))
            .unwrap_or(frame);
        let location = colon_location(location)?;
        let path = location.0.to_str()?;
        (!path.starts_with("node:")
            && !path.contains("://")
            && ["js", "cjs", "mjs", "ts", "tsx", "jsx"]
                .contains(&location.0.extension()?.to_str()?))
        .then_some(location)
    });
    // A bare TypeError also occurs in Python; require a JS frame or a Node code.
    if location.is_none()
        && !explicit_code.is_some_and(|c| c.starts_with("ERR_") || c == "MODULE_NOT_FOUND")
    {
        return None;
    }
    let (file, line, column) =
        location.map_or((None, None, None), |(f, l, c)| (Some(f), Some(l), Some(c)));
    Some(Diagnostic {
        source: Some("node".into()),
        code: Some(explicit_code.unwrap_or(kind).into()),
        message: message.into(),
        file,
        line,
        column,
    })
}

fn parse_python_exception(line: &str) -> Option<(String, String)> {
    let (name, message) = line.split_once(':').unwrap_or((line, ""));
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        || !(name.ends_with("Error")
            || name.ends_with("Exception")
            || matches!(
                name,
                "KeyboardInterrupt" | "SystemExit" | "StopIteration" | "ExceptionGroup"
            ))
    {
        return None;
    }
    Some((
        name.to_owned(),
        if message.trim().is_empty() {
            name.to_owned()
        } else {
            message.trim().to_owned()
        },
    ))
}

fn parse_error_header(line: &str) -> Option<(Option<String>, String)> {
    let line = line.trim_start();
    if let Some(rest) = line.strip_prefix("error[") {
        let (code, message) = rest.split_once("]:")?;
        if code.is_empty() || message.trim().is_empty() {
            return None;
        }
        return Some((Some(code.to_owned()), message.trim().to_owned()));
    }
    line.strip_prefix("error:")
        .map(|message| (None, message.trim().to_owned()))
        .filter(|(_, message)| !message.is_empty())
}

fn parse_location(line: &str) -> Option<(PathBuf, u32, u32)> {
    let location = line.trim_start().strip_prefix("-->")?.trim();
    let mut parts = location.rsplitn(3, ':');
    let column = positive(parts.next()?)?;
    let line = positive(parts.next()?)?;
    let file = parts.next()?.trim();
    if file.is_empty() {
        return None;
    }
    Some((PathBuf::from(file), line, column))
}

fn classify_source(
    code: Option<&str>,
    file: Option<&std::path::Path>,
    message: &str,
) -> Option<String> {
    if code.is_some()
        || file
            .and_then(|path| path.extension())
            .and_then(|extension| extension.to_str())
            == Some("rs")
    {
        Some("rustc".to_owned())
    } else if message.contains("could not compile") || message.contains("failed to get") {
        Some("cargo".to_owned())
    } else {
        None
    }
}

fn strip_ansi(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut characters = input.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\u{1b}' && characters.peek() == Some(&'[') {
            characters.next();
            for escape_character in characters.by_ref() {
                if escape_character.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            output.push(character);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use anyhow::{Context, Result};

    use super::*;

    #[test]
    fn broader_compilers_preserve_windows_paths_unicode_crlf_and_multiple_errors() {
        let parsed = parse_diagnostics(
            "error: could not compile\r\n\x1b[31mC:\\work\\งาน.ts(2,3): error TS2322: incompatible type\x1b[0m\r\nsrc/view.ts:4:5 - error TS2307: cannot find module\r\nmain.go:6:7: undefined: missing\r\nother.go:8: invalid operation\r\nTypeError: bad call\r\n    at run (C:\\work\\app.js:9:10)\r\nError [ERR_MODULE_NOT_FOUND]: Cannot find package\r\n",
        );
        assert_eq!(parsed.len(), 7, "{parsed:?}");
        assert_eq!(parsed[1].source.as_deref(), Some("typescript"));
        assert_eq!(parsed[1].file, Some(PathBuf::from(r"C:\work\งาน.ts")));
        assert_eq!(parsed[2].code.as_deref(), Some("TS2307"));
        assert_eq!(parsed[3].source.as_deref(), Some("go"));
        assert_eq!(parsed[4].column, None);
        assert_eq!(parsed[5].source.as_deref(), Some("node"));
        assert_eq!(parsed[5].line, Some(9));
        assert_eq!(parsed[6].file, None);
    }

    #[test]
    fn malformed_broader_diagnostics_do_not_invent_structure() {
        for log in [
            "a.ts(0,1): error TS2322: bad",
            "a.ts(1,-1): error TS2322: bad",
            "a.ts(1,4294967296): error TS2322: bad",
            "a.ts(1,2): error TS: bad",
            "a.go:0:1: bad",
            "TypeError: bad\n at app.js:bad:3",
            "TypeError: bad\n at node:internal/main:1:2",
        ] {
            let parsed = parse_primary(log).unwrap();
            assert_eq!(parsed.source, None, "{log}");
            assert_eq!(parsed.file, None, "{log}");
        }
    }

    #[test]
    fn python_traceback_uses_last_frame_and_final_exception() {
        let input = "Traceback (most recent call last):\n  File \"runner.py\", line 9, in main\n    run()\n  File \"src/งาน.py\", line 3, in run\n    x[0]\n\x1b[31mIndexError: list index out of range\x1b[0m";
        let diagnostic = parse_primary(input).unwrap();
        assert_eq!(diagnostic.source.as_deref(), Some("python"));
        assert_eq!(diagnostic.code.as_deref(), Some("IndexError"));
        assert_eq!(diagnostic.file, Some("src/งาน.py".into()));
        assert_eq!(diagnostic.line, Some(3));
    }

    #[test]
    fn python_chained_errors_and_syntax_errors_remain_separate() {
        let input = "Traceback (most recent call last):\n  File \"a.py\", line 1\nValueError: bad\nDuring handling of the above exception, another exception occurred:\nTraceback (most recent call last):\n  File \"b.py\", line 2\nCustomError: failed\n  File \"syntax.py\", line 4\nSyntaxError: invalid syntax";
        let parsed = parse_diagnostics(input);
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].code.as_deref(), Some("ValueError"));
        assert_eq!(parsed[1].file, Some("b.py".into()));
        assert_eq!(parsed[2].code.as_deref(), Some("SyntaxError"));
    }

    #[test]
    fn malformed_python_frames_do_not_invent_locations() {
        for number in ["0", "-1", "4294967296", "bad"] {
            let input = format!(
                "Traceback (most recent call last):\n  File \"a.py\", line {number}\nValueError: bad"
            );
            let parsed = parse_primary(&input).unwrap();
            assert_eq!(parsed.file, None);
            assert_eq!(parsed.line, None);
        }
        let parsed = parse_primary("File \"a.py\", line nonsense\nsomething unknown").unwrap();
        assert_eq!(parsed.source, None);
        assert!(parsed.message.contains("nonsense"));
    }

    #[test]
    fn rust_without_location_does_not_consume_a_following_python_traceback() {
        let parsed = parse_diagnostics(
            "error: failed to get dependency\nTraceback (most recent call last):\n  File \"worker.py\", line 1\nRuntimeError: failed",
        );
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].source.as_deref(), Some("cargo"));
        assert_eq!(parsed[1].source.as_deref(), Some("python"));
    }

    #[test]
    fn parses_rust_error_code_and_location() -> Result<()> {
        let input = r#"error[E0382]: borrow of moved value: `name`
 --> src/main.rs:12:20
  |
8 |     consume(name);
  |             ---- value moved here"#;
        let diagnostic = parse_primary(input).context("expected a diagnostic")?;
        assert_eq!(diagnostic.source.as_deref(), Some("rustc"));
        assert_eq!(diagnostic.code.as_deref(), Some("E0382"));
        assert_eq!(diagnostic.message, "borrow of moved value: `name`");
        assert_eq!(diagnostic.file, Some(PathBuf::from("src/main.rs")));
        assert_eq!(diagnostic.line, Some(12));
        assert_eq!(diagnostic.column, Some(20));
        Ok(())
    }

    #[test]
    fn strips_color_codes_before_parsing() -> Result<()> {
        let input = "\u{1b}[31merror[E0432]\u{1b}[0m: unresolved import `tokio`\n \u{1b}[34m-->\u{1b}[0m src/main.rs:1:5";
        let diagnostic = parse_primary(input).context("expected a diagnostic")?;
        assert_eq!(diagnostic.code.as_deref(), Some("E0432"));
        assert_eq!(diagnostic.line, Some(1));
        Ok(())
    }

    #[test]
    fn keeps_unknown_input_without_inventing_structure() -> Result<()> {
        let diagnostic = parse_primary("connection terminated unexpectedly")
            .context("expected an unknown diagnostic")?;
        assert_eq!(diagnostic.source, None);
        assert_eq!(diagnostic.code, None);
        assert_eq!(diagnostic.message, "connection terminated unexpectedly");
        assert_eq!(diagnostic.file, None);
        Ok(())
    }
}
