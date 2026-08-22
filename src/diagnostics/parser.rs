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

    while index < lines.len() {
        let Some((code, message)) = parse_error_header(lines[index]) else {
            index += 1;
            continue;
        };
        let mut file = None;
        let mut line = None;
        let mut column = None;
        let mut cursor = index + 1;
        while cursor < lines.len() && parse_error_header(lines[cursor]).is_none() {
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
    let column = parts.next()?.parse().ok()?;
    let line = parts.next()?.parse().ok()?;
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
